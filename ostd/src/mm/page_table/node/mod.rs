// SPDX-License-Identifier: MPL-2.0

//! This module defines page table node abstractions and the handle.
//!
//! The page table node is also frequently referred to as a page table in many architectural
//! documentations. It is essentially a page that contains page table entries (PTEs) that map
//! to child page tables nodes or mapped pages.
//!
//! This module leverages the page metadata to manage the page table pages, which makes it
//! easier to provide the following guarantees:
//!
//! The page table node is not freed when it is still in use by:
//!    - a parent page table node,
//!    - or a handle to a page table node,
//!    - or a processor.
//!
//! This is implemented by using a reference counter in the page metadata. If the above
//! conditions are not met, the page table node is ensured to be freed upon dropping the last
//! reference.
//!
//! One can acquire exclusive access to a page table node using merely the physical address of
//! the page table node. This is implemented by a lock in the page metadata. Here the
//! exclusiveness is only ensured for kernel code, and the processor's MMU is able to access the
//! page table node while a lock is held. So the modification to the PTEs should be done after
//! the initialization of the entity that the PTE points to. This is taken care in this module.
//!

mod child;

use core::{fmt, marker::PhantomData, mem::ManuallyDrop, panic, sync::atomic::Ordering};

pub(in crate::mm) use child::Child;

use super::{nr_subpage_per_huge, page_size, PageTableEntryTrait};
use crate::{
    arch::mm::{PageTableEntry, PagingConsts},
    mm::{
        paddr_to_vaddr,
        page::{
            self,
            meta::{MapTrackingStatus, PageMeta, PageTablePageMeta, PageUsage},
            DynPage, Page,
        },
        page_prop::PageProperty,
        Paddr, PagingConstsTrait, PagingLevel, PAGE_SIZE,
    },
};

/// The raw handle to a page table node.
///
/// This handle is a referencer of a page table node. Thus creating and dropping it will affect
/// the reference count of the page table node. If dropped the raw handle as the last reference,
/// the page table node and subsequent children will be freed.
///
/// Only the CPU or a PTE can access a page table node using a raw handle. To access the page
/// table node from the kernel code, use the handle [`PageTableNode`].
#[derive(Debug)]
pub(super) struct RawPageTableNode<E: PageTableEntryTrait, C: PagingConstsTrait>
where
    [(); C::NR_LEVELS as usize]:,
{
    pub(super) raw: Paddr,
    _phantom: PhantomData<(E, C)>,
}

impl<E: PageTableEntryTrait, C: PagingConstsTrait> RawPageTableNode<E, C>
where
    [(); C::NR_LEVELS as usize]:,
{
    pub(super) fn paddr(&self) -> Paddr {
        self.raw
    }

    /// Converts a raw handle to an accessible handle by pertaining the lock.
    pub(super) fn lock(self) -> PageTableNode<E, C> {
        // Prevent dropping the handle.
        let this = ManuallyDrop::new(self);

        // SAFETY: The physical address in the raw handle is valid and we are
        // transferring the ownership to a new handle. No increment of the reference
        // count is needed.
        let page = unsafe { Page::<PageTablePageMeta<E, C>>::from_raw(this.paddr()) };

        // Acquire the lock.
        while page
            .meta()
            .lock
            .compare_exchange(0, 1, Ordering::Acquire, Ordering::Relaxed)
            .is_err()
        {
            core::hint::spin_loop();
        }

        PageTableNode::<E, C> { page, _private: () }
    }

    /// Creates a copy of the handle.
    pub(super) fn clone_shallow(&self) -> Self {
        self.inc_ref_count();

        Self {
            raw: self.raw,
            _phantom: PhantomData,
        }
    }

    /// Activates the page table assuming it is a root page table.
    ///
    /// Here we ensure not dropping an active page table by making a
    /// processor a page table owner. When activating a page table, the
    /// reference count of the last activated page table is decremented.
    /// And that of the current page table is incremented.
    ///
    /// # Safety
    ///
    /// The caller must ensure that the page table to be activated has
    /// proper mappings for the kernel and has the correct const parameters
    /// matching the current CPU.
    pub(crate) unsafe fn activate(&self) {
        use crate::{
            arch::mm::{activate_page_table, current_page_table_paddr},
            mm::CachePolicy,
        };

        let last_activated_paddr = current_page_table_paddr();

        if last_activated_paddr == self.raw {
            return;
        }

        activate_page_table(self.raw, CachePolicy::Writeback);

        // Increment the reference count of the current page table.
        self.inc_ref_count();

        // Restore and drop the last activated page table.
        drop(Self {
            raw: last_activated_paddr,
            _phantom: PhantomData,
        });
    }

    /// Activates the (root) page table assuming it is the first activation.
    ///
    /// It will not try dropping the last activate page table. It is the same
    /// with [`Self::activate()`] in other senses.
    pub(super) unsafe fn first_activate(&self) {
        use crate::{arch::mm::activate_page_table, mm::CachePolicy};

        self.inc_ref_count();

        activate_page_table(self.raw, CachePolicy::Writeback);
    }

    fn inc_ref_count(&self) {
        // SAFETY: We have a reference count to the page and can safely increase the reference
        // count by one more.
        unsafe {
            Page::<PageTablePageMeta<E, C>>::inc_ref_count(self.paddr());
        }
    }

    /// Restore the handle to a page table node from a physical address.
    ///
    /// # Safety
    ///
    /// The caller must ensure that the physical address is valid and points to
    /// a forgotten page table node. A forgotten page table node can only be
    /// restored once.
    unsafe fn from_paddr(paddr: Paddr) -> Self {
        Self {
            raw: paddr,
            _phantom: PhantomData,
        }
    }
}

impl<E: PageTableEntryTrait, C: PagingConstsTrait> Drop for RawPageTableNode<E, C>
where
    [(); C::NR_LEVELS as usize]:,
{
    fn drop(&mut self) {
        // SAFETY: The physical address in the raw handle is valid. The restored
        // handle is dropped to decrement the reference count.
        drop(unsafe { Page::<PageTablePageMeta<E, C>>::from_raw(self.paddr()) });
    }
}

/// A mutable handle to a page table node.
///
/// The page table node can own a set of handles to children, ensuring that the children
/// don't outlive the page table node. Cloning a page table node will create a deep copy
/// of the page table. Dropping the page table node will also drop all handles if the page
/// table node has no references. You can set the page table node as a child of another
/// page table node.
pub(super) struct PageTableNode<
    E: PageTableEntryTrait = PageTableEntry,
    C: PagingConstsTrait = PagingConsts,
> where
    [(); C::NR_LEVELS as usize]:,
{
    pub(super) page: Page<PageTablePageMeta<E, C>>,
    _private: (),
}

// FIXME: We cannot `#[derive(Debug)]` here due to `DisabledPreemptGuard`. Should we skip
// this field or implement the `Debug` trait also for `DisabledPreemptGuard`?
impl<E, C> fmt::Debug for PageTableNode<E, C>
where
    E: PageTableEntryTrait,
    C: PagingConstsTrait,
    [(); C::NR_LEVELS as usize]:,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("PageTableEntryTrait")
            .field("page", &self.page)
            .finish()
    }
}

impl<E: PageTableEntryTrait, C: PagingConstsTrait> PageTableNode<E, C>
where
    [(); C::NR_LEVELS as usize]:,
{
    /// Allocates a new empty page table node.
    ///
    /// This function returns an owning handle. The newly created handle does not
    /// set the lock bit for performance as it is exclusive and unlocking is an
    /// extra unnecessary expensive operation.
    pub(super) fn alloc(level: PagingLevel, is_tracked: MapTrackingStatus) -> Self {
        let meta = PageTablePageMeta::new_locked(level, is_tracked);
        let page = page::allocator::alloc_single::<PageTablePageMeta<E, C>>(meta).unwrap();

        // Zero out the page table node.
        let ptr = paddr_to_vaddr(page.paddr()) as *mut u8;
        // SAFETY: The page is exclusively owned here. Pointers are valid also.
        // We rely on the fact that 0 represents an absent entry to speed up `memset`.
        unsafe { core::ptr::write_bytes(ptr, 0, PAGE_SIZE) };
        debug_assert!(E::new_absent().as_bytes().iter().all(|&b| b == 0));

        Self { page, _private: () }
    }

    pub fn level(&self) -> PagingLevel {
        self.meta().level
    }

    pub fn is_tracked(&self) -> MapTrackingStatus {
        self.meta().is_tracked
    }

    /// Converts the handle into a raw handle to be stored in a PTE or CPU.
    pub(super) fn into_raw(self) -> RawPageTableNode<E, C> {
        let this = ManuallyDrop::new(self);

        let raw = this.page.paddr();

        this.page.meta().lock.store(0, Ordering::Release);

        RawPageTableNode {
            raw,
            _phantom: PhantomData,
        }
    }

    /// Gets a raw handle while still preserving the original handle.
    pub(super) fn clone_raw(&self) -> RawPageTableNode<E, C> {
        core::mem::forget(self.page.clone());

        RawPageTableNode {
            raw: self.page.paddr(),
            _phantom: PhantomData,
        }
    }

    /// Gets an extra reference of the child at the given index.
    pub(super) fn child(&self, idx: usize) -> Child<E, C> {
        debug_assert!(idx < nr_subpage_per_huge::<C>());

        let pte = self.read_pte(idx);

        // SAFETY: The PTE is read from this page table node so the information
        // recorded in this page table is correct.
        unsafe { Child::clone_from_pte(&pte, self.level(), self.is_tracked()) }
    }

    /// Replace the child at the given index with a new child.
    ///
    /// The old child is returned. The new child must match the level of the page
    /// table node and the tracking status of the page table node.
    pub(super) fn replace_child(&mut self, idx: usize, new_child: Child<E, C>) -> Child<E, C> {
        // It should be ensured by the cursor.
        #[cfg(debug_assertions)]
        match &new_child {
            Child::PageTable(_) => {
                debug_assert!(self.level() > 1);
            }
            Child::Page(p, _) => {
                debug_assert!(self.level() == p.level());
                debug_assert!(self.is_tracked() == MapTrackingStatus::Tracked);
            }
            Child::Untracked(_, level, _) => {
                debug_assert!(self.level() == *level);
                debug_assert!(self.is_tracked() == MapTrackingStatus::Untracked);
            }
            Child::None => {}
        }

        let pte = self.read_pte(idx);
        // SAFETY: The PTE is read from this page table node so the information
        // provided is correct. The PTE is not restored twice.
        let old_child = unsafe { Child::from_pte(pte, self.level(), self.is_tracked()) };

        if old_child.is_none() && !new_child.is_none() {
            *self.nr_children_mut() += 1;
        } else if !old_child.is_none() && new_child.is_none() {
            *self.nr_children_mut() -= 1;
        }

        self.write_pte(idx, new_child.into_pte());

        old_child
    }

    /// Splits the untracked huge page mapped at `idx` to smaller pages.
    pub(super) fn split_untracked_huge(&mut self, idx: usize) {
        // These should be ensured by the cursor.
        debug_assert!(idx < nr_subpage_per_huge::<C>());
        debug_assert!(self.level() > 1);

        let Child::Untracked(pa, level, prop) = self.child(idx) else {
            panic!("`split_untracked_huge` not called on an untracked huge page");
        };

        debug_assert_eq!(level, self.level());

        let mut new_page = PageTableNode::<E, C>::alloc(level - 1, MapTrackingStatus::Untracked);
        for i in 0..nr_subpage_per_huge::<C>() {
            let small_pa = pa + i * page_size::<C>(level - 1);
            new_page.replace_child(i, Child::Untracked(small_pa, level - 1, prop));
        }

        self.replace_child(idx, Child::PageTable(new_page.into_raw()));
    }

    /// Protects an already mapped child at a given index.
    pub(super) fn protect(&mut self, idx: usize, prop: PageProperty) {
        let mut pte = self.read_pte(idx);
        debug_assert!(pte.is_present()); // This should be ensured by the cursor.

        pte.set_prop(prop);

        // SAFETY: the index is within the bound and the PTE is valid.
        unsafe {
            (self.as_ptr() as *mut E).add(idx).write(pte);
        }
    }

    pub(super) fn read_pte(&self, idx: usize) -> E {
        // It should be ensured by the cursor.
        debug_assert!(idx < nr_subpage_per_huge::<C>());

        // SAFETY: the index is within the bound and PTE is plain-old-data.
        unsafe { self.as_ptr().add(idx).read() }
    }

    /// Writes a page table entry at a given index.
    ///
    /// This operation will leak the old child if the PTE is present.
    fn write_pte(&mut self, idx: usize, pte: E) {
        // It should be ensured by the cursor.
        debug_assert!(idx < nr_subpage_per_huge::<C>());

        // SAFETY: the index is within the bound and PTE is plain-old-data.
        unsafe { (self.as_ptr() as *mut E).add(idx).write(pte) };
    }

    /// The number of valid PTEs.
    pub(super) fn nr_children(&self) -> u16 {
        // SAFETY: The lock is held so there is no mutable reference to it.
        // It would be safe to read.
        unsafe { *self.meta().nr_children.get() }
    }

    fn nr_children_mut(&mut self) -> &mut u16 {
        // SAFETY: The lock is held so we have an exclusive access.
        unsafe { &mut *self.meta().nr_children.get() }
    }

    fn as_ptr(&self) -> *const E {
        paddr_to_vaddr(self.start_paddr()) as *const E
    }

    fn start_paddr(&self) -> Paddr {
        self.page.paddr()
    }

    fn meta(&self) -> &PageTablePageMeta<E, C> {
        self.page.meta()
    }
}

impl<E: PageTableEntryTrait, C: PagingConstsTrait> Drop for PageTableNode<E, C>
where
    [(); C::NR_LEVELS as usize]:,
{
    fn drop(&mut self) {
        // Release the lock.
        self.page.meta().lock.store(0, Ordering::Release);
    }
}

impl<E: PageTableEntryTrait, C: PagingConstsTrait> PageMeta for PageTablePageMeta<E, C>
where
    [(); C::NR_LEVELS as usize]:,
{
    const USAGE: PageUsage = PageUsage::PageTable;

    fn on_drop(page: &mut Page<Self>) {
        let paddr = page.paddr();
        let level = page.meta().level;
        let is_tracked = page.meta().is_tracked;

        // Drop the children.
        for i in 0..nr_subpage_per_huge::<C>() {
            // SAFETY: The index is within the bound and PTE is plain-old-data. The
            // address is aligned as well. We also have an exclusive access ensured
            // by reference counting.
            let pte_ptr = unsafe { (paddr_to_vaddr(paddr) as *const E).add(i) };
            // SAFETY: The pointer is valid and the PTE is plain-old-data.
            let pte = unsafe { pte_ptr.read() };

            // Here if we use directly `Child::from_pte` we would experience a
            // 50% increase in the overhead of the `drop` function. It seems that
            // Rust is very conservative about inlining and optimizing dead code
            // for `unsafe` code. So we manually inline the function here.
            if pte.is_present() {
                let paddr = pte.paddr();
                if !pte.is_last(level) {
                    // SAFETY: The PTE points to a page table node. The ownership
                    // of the child is transferred to the child then dropped.
                    drop(unsafe { Page::<Self>::from_raw(paddr) });
                } else if is_tracked == MapTrackingStatus::Tracked {
                    // SAFETY: The PTE points to a tracked page. The ownership
                    // of the child is transferred to the child then dropped.
                    drop(unsafe { DynPage::from_raw(paddr) });
                }
            }
        }
    }
}