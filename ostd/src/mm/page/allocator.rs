// SPDX-License-Identifier: MPL-2.0

//! The physical page memory allocator.
//!
//! TODO: Decouple it with the frame allocator in [`crate::mm::frame::options`] by
//! allocating pages rather untyped memory from this module.

use alloc::vec::Vec;

use align_ext::AlignExt;
use buddy_system_allocator::FrameAllocator;
use log::info;
use spin::Once;

use super::{cont_pages::ContPages, meta::PageMeta, Page};
use crate::{boot::memory_region::MemoryRegionType, mm::PAGE_SIZE, sync::SpinLock};

pub(in crate::mm) static PAGE_ALLOCATOR: Once<SpinLock<FrameAllocator>> = Once::new();

/// Allocate a single page.
pub(crate) fn alloc_single<M: PageMeta>() -> Option<Page<M>> {
    PAGE_ALLOCATOR.get().unwrap().lock().alloc(1).map(|idx| {
        let paddr = idx * PAGE_SIZE;
        Page::<M>::from_unused(paddr)
    })
}

/// Allocate a contiguous range of pages of a given length in bytes.
///
/// # Panics
///
/// The function panics if the length is not base-page-aligned.
pub(crate) fn alloc_contiguous<M: PageMeta>(len: usize) -> Option<ContPages<M>> {
    assert!(len % PAGE_SIZE == 0);
    PAGE_ALLOCATOR
        .get()
        .unwrap()
        .lock()
        .alloc(len / PAGE_SIZE)
        .map(|start| ContPages::from_unused(start * PAGE_SIZE..start * PAGE_SIZE + len))
}

/// Allocate pages.
///
/// The allocated pages are not guarenteed to be contiguous.
/// The total length of the allocated pages is `len`.
///
/// # Panics
///
/// The function panics if the length is not base-page-aligned.
pub(crate) fn alloc<M: PageMeta>(len: usize) -> Option<Vec<Page<M>>> {
    assert!(len % PAGE_SIZE == 0);
    let nframes = len / PAGE_SIZE;
    let mut allocator = PAGE_ALLOCATOR.get().unwrap().lock();
    let mut vector = Vec::new();
    for _ in 0..nframes {
        let paddr = allocator.alloc(1)? * PAGE_SIZE;
        let page = Page::<M>::from_unused(paddr);
        vector.push(page);
    }
    Some(vector)
}

pub(crate) fn init() {
    let regions = crate::boot::memory_regions();
    let mut allocator = FrameAllocator::<32>::new();
    for region in regions.iter() {
        if region.typ() == MemoryRegionType::Usable {
            // Make the memory region page-aligned, and skip if it is too small.
            let start = region.base().align_up(PAGE_SIZE) / PAGE_SIZE;
            let region_end = region.base().checked_add(region.len()).unwrap();
            let end = region_end.align_down(PAGE_SIZE) / PAGE_SIZE;
            if end <= start {
                continue;
            }
            // Add global free pages to the frame allocator.
            allocator.add_frame(start, end);
            info!(
                "Found usable region, start:{:x}, end:{:x}",
                region.base(),
                region.base() + region.len()
            );
        }
    }
    PAGE_ALLOCATOR.call_once(|| SpinLock::new(allocator));
}
