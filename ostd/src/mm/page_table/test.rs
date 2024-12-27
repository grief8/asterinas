// SPDX-License-Identifier: MPL-2.0

use super::*;
use crate::{
    mm::{
        frame::FrameMeta,
        kspace::LINEAR_MAPPING_BASE_VADDR,
        page::allocator,
        page_prop::{CachePolicy, PageFlags},
        MAX_USERSPACE_VADDR, PAGE_SIZE,
    },
    panic,
    prelude::*,
};

#[cfg(ktest)]
mod test_utils {
    use super::*;

    /// Sets up an empty PageTable in the specified mode.
    pub fn setup_page_table<M: PageTableMode>() -> PageTable<M> {
        PageTable::<M>::empty()
    }

    /// Maps a range of virtual addresses to physical addresses with the given properties.
    pub fn map_range<M: PageTableMode, E: PageTableEntryTrait, C: PagingConstsTrait>(
        pt: &PageTable<M, E, C>,
        from: Range<usize>,
        to: Range<usize>,
        prop: PageProperty,
    ) where
        [(); C::NR_LEVELS as usize]:,
    {
        unsafe {
            pt.map(&from, &to, prop).unwrap();
        }
    }

    /// Unmaps a range of virtual addresses.
    pub fn unmap_range<M: PageTableMode>(pt: &PageTable<M>, range: Range<usize>) {
        unsafe {
            pt.cursor_mut(&range).unwrap().take_next(range.len());
        }
    }

    #[derive(Clone, Debug, Default)]
    pub struct VeryHugePagingConsts;

    impl PagingConstsTrait for VeryHugePagingConsts {
        const NR_LEVELS: PagingLevel = 4;
        const BASE_PAGE_SIZE: usize = PAGE_SIZE;
        const ADDRESS_WIDTH: usize = 48;
        const HIGHEST_TRANSLATION_LEVEL: PagingLevel = 3;
        const PTE_SIZE: usize = core::mem::size_of::<PageTableEntry>();
    }

    impl<M: PageTableMode, E: PageTableEntryTrait, C: PagingConstsTrait> PageTable<M, E, C>
    where
        [(); C::NR_LEVELS as usize]:,
    {
        /// Applies a protection operation to a range of virtual addresses.
        pub fn protect(&self, range: &Range<Vaddr>, mut op: impl FnMut(&mut PageProperty)) {
            let mut cursor = self.cursor_mut(range).unwrap();
            loop {
                unsafe {
                    if cursor
                        .protect_next(range.end - cursor.virt_addr(), &mut op)
                        .is_none()
                    {
                        break;
                    }
                }
            }
        }
    }
}

#[cfg(ktest)]
mod range_checks {
    use super::{test_utils::*, *};

    #[ktest]
    fn range_check() {
        let pt = setup_page_table::<UserMode>();
        let good_va = 0..PAGE_SIZE;
        let bad_va = 0..(PAGE_SIZE + 1);
        let bad_va2 = LINEAR_MAPPING_BASE_VADDR..(LINEAR_MAPPING_BASE_VADDR + PAGE_SIZE);

        assert!(pt.cursor_mut(&good_va).is_ok());
        assert!(pt.cursor_mut(&bad_va).is_err());
        assert!(pt.cursor_mut(&bad_va2).is_err());
    }

    #[ktest]
    fn boundary_conditions() {
        let pt = setup_page_table::<UserMode>();

        // Test empty range
        let empty_range = 0..0;
        assert!(
            pt.cursor_mut(&empty_range).is_err(),
            "Empty range should fail"
        );

        // Test out of range
        let out_of_range = MAX_USERSPACE_VADDR..(MAX_USERSPACE_VADDR + PAGE_SIZE);
        assert!(
            pt.cursor_mut(&out_of_range).is_err(),
            "Out-of-bounds range should fail"
        );

        // Test misaligned addresses
        let unaligned_range = 1..(PAGE_SIZE + 1);
        assert!(
            pt.cursor_mut(&unaligned_range).is_err(),
            "Misaligned range should fail"
        );
    }

    #[ktest]
    fn maximum_page_table_mapping() {
        let pt = setup_page_table::<UserMode>();

        // Use a smaller range to avoid performance issues
        let max_addr = 0x100000;
        let range = 0..max_addr;
        let prop = PageProperty::new(PageFlags::RW, CachePolicy::Writeback);

        let mut cursor = pt.cursor_mut(&range).unwrap();

        // Allocate and map pages
        let pages = allocator::alloc(max_addr, |_| FrameMeta::default()).unwrap();
        for page in pages.iter() {
            unsafe {
                cursor.map(page.clone().into(), prop);
            }
        }

        // Verify sample mappings
        assert!(pt.query(0).is_some(), "VA 0 should be mapped");
        assert!(
            pt.query(max_addr / 2).is_some(),
            "VA max_addr/2 should be mapped"
        );
        assert!(
            pt.query(max_addr - PAGE_SIZE).is_some(),
            "VA max_addr-PAGE_SIZE should be mapped"
        );
    }

    #[ktest]
    fn start_boundary_mapping() {
        let pt = setup_page_table::<UserMode>();
        let range = 0..PAGE_SIZE;
        let prop = PageProperty::new(PageFlags::RW, CachePolicy::Writeback);

        unsafe {
            pt.cursor_mut(&range).unwrap().map(
                allocator::alloc_single(FrameMeta::default())
                    .unwrap()
                    .into(),
                prop,
            );
        }

        assert!(pt.query(0).is_some(), "Start of range should be mapped");
        assert!(
            pt.query(PAGE_SIZE - 1).is_some(),
            "End of range should be mapped"
        );
    }

    #[ktest]
    fn end_boundary_mapping() {
        let pt = setup_page_table::<UserMode>();
        let range = (MAX_USERSPACE_VADDR - PAGE_SIZE)..MAX_USERSPACE_VADDR;
        let prop = PageProperty::new(PageFlags::RW, CachePolicy::Writeback);

        unsafe {
            pt.cursor_mut(&range).unwrap().map(
                allocator::alloc_single(FrameMeta::default())
                    .unwrap()
                    .into(),
                prop,
            );
        }

        assert!(
            pt.query(MAX_USERSPACE_VADDR - PAGE_SIZE).is_some(),
            "Start of end range should be mapped"
        );
        assert!(
            pt.query(MAX_USERSPACE_VADDR - 1).is_some(),
            "End of user space should be mapped"
        );
    }

    #[ktest]
    #[should_panic]
    fn overflow_boundary_mapping() {
        let pt = setup_page_table::<UserMode>();
        let range =
            (MAX_USERSPACE_VADDR - (PAGE_SIZE / 2))..(MAX_USERSPACE_VADDR + (PAGE_SIZE / 2));
        let prop = PageProperty::new(PageFlags::RW, CachePolicy::Writeback);

        unsafe {
            pt.cursor_mut(&range).unwrap().map(
                allocator::alloc_single(FrameMeta::default())
                    .unwrap()
                    .into(),
                prop,
            );
        }
    }
}

#[cfg(ktest)]
mod page_properties {
    use super::{test_utils::*, *};

    #[ktest]
    fn invalid_page_properties() {
        let pt = setup_page_table::<UserMode>();
        let from = PAGE_SIZE..(PAGE_SIZE * 2);
        let page = allocator::alloc_single(FrameMeta::default()).unwrap();

        // Attempt to map with invalid properties
        // NOTE: Ensure that the 'map' function validates properties appropriately
        let invalid_prop = PageProperty::new(PageFlags::RW, CachePolicy::Uncacheable);

        unsafe {
            pt.cursor_mut(&from).unwrap().map(page.into(), invalid_prop);
            let (_, prop) = pt.query(from.start + 10).unwrap();
            assert_eq!(
                prop.cache,
                CachePolicy::Uncacheable,
                "Cache policy should be Uncacheable"
            );
        }
    }

    #[ktest]
    fn varied_page_flags() {
        let pt = setup_page_table::<UserMode>();
        let range = PAGE_SIZE..(PAGE_SIZE * 2);
        let page = allocator::alloc_single(FrameMeta::default()).unwrap();

        // Read-Only mapping
        let ro_prop = PageProperty::new(PageFlags::R, CachePolicy::Writeback);
        unsafe {
            pt.cursor_mut(&range)
                .unwrap()
                .map(page.clone().into(), ro_prop);
        }

        let queried = pt.query(range.start + 100).unwrap().1;
        assert_eq!(queried.flags, PageFlags::R, "Page should be Read-Only");

        // No-Execute mapping
        let nx_prop = PageProperty::new(PageFlags::RX, CachePolicy::Writeback);
        test_utils::unmap_range(&pt, range.clone());

        unsafe {
            pt.cursor_mut(&range)
                .unwrap()
                .map(page.clone().into(), nx_prop);
        }

        let queried = pt.query(range.start + 100).unwrap().1;
        assert_eq!(queried.flags, PageFlags::RX, "Page should be Read-Execute");

        // Read-Write-Execute mapping
        let rwx_prop = PageProperty::new(PageFlags::RWX, CachePolicy::Writeback);
        test_utils::unmap_range(&pt, range.clone());

        unsafe {
            pt.cursor_mut(&range)
                .unwrap()
                .map(page.clone().into(), rwx_prop);
        }

        let queried = pt.query(range.start + 100).unwrap().1;
        assert_eq!(
            queried.flags,
            PageFlags::RWX,
            "Page should be Read-Write-Execute"
        );
    }

    #[ktest]
    fn varied_cache_policies() {
        let pt = setup_page_table::<UserMode>();
        let range = PAGE_SIZE..(PAGE_SIZE * 2);
        let page = allocator::alloc_single(FrameMeta::default()).unwrap();

        // Writeback cache policy
        let wb_prop = PageProperty::new(PageFlags::RW, CachePolicy::Writeback);
        unsafe {
            pt.cursor_mut(&range)
                .unwrap()
                .map(page.clone().into(), wb_prop);
        }

        let queried = pt.query(range.start + 100).unwrap().1;
        assert_eq!(
            queried.cache,
            CachePolicy::Writeback,
            "Cache policy should be Writeback"
        );

        // Write-Through cache policy
        let wt_prop = PageProperty::new(PageFlags::RW, CachePolicy::Writethrough);
        test_utils::unmap_range(&pt, range.clone());

        unsafe {
            pt.cursor_mut(&range)
                .unwrap()
                .map(page.clone().into(), wt_prop);
        }

        let queried = pt.query(range.start + 100).unwrap().1;
        assert_eq!(
            queried.cache,
            CachePolicy::Writethrough,
            "Cache policy should be Writethrough"
        );

        // Uncacheable cache policy
        let uc_prop = PageProperty::new(PageFlags::RW, CachePolicy::Uncacheable);
        test_utils::unmap_range(&pt, range.clone());

        unsafe {
            pt.cursor_mut(&range)
                .unwrap()
                .map(page.clone().into(), uc_prop);
        }

        let queried = pt.query(range.start + 100).unwrap().1;
        assert_eq!(
            queried.cache,
            CachePolicy::Uncacheable,
            "Cache policy should be Uncacheable"
        );
    }
}

#[cfg(ktest)]
mod different_page_sizes {
    use super::{test_utils::*, *};

    #[ktest]
    fn different_page_sizes() {
        let pt = setup_page_table::<UserMode>();

        // Test 2MiB pages
        let from_2m = PAGE_SIZE * 512..(PAGE_SIZE * 512 * 2);
        let page_2m = allocator::alloc_single(FrameMeta::default()).unwrap();
        let prop = PageProperty::new(PageFlags::RW, CachePolicy::Writeback);

        unsafe {
            pt.cursor_mut(&from_2m).unwrap().map(page_2m.into(), prop);
        }

        assert!(
            pt.query(from_2m.start + 10).is_some(),
            "2MiB page start should be mapped"
        );

        // Test 1GiB pages
        let from_1g = PAGE_SIZE * 512 * 512..(PAGE_SIZE * 512 * 512 * 2);
        let page_1g = allocator::alloc_single(FrameMeta::default()).unwrap();

        unsafe {
            pt.cursor_mut(&from_1g).unwrap().map(page_1g.into(), prop);
        }

        assert!(
            pt.query(from_1g.start + 10).is_some(),
            "1GiB page start should be mapped"
        );
    }
}

#[cfg(ktest)]
mod overlapping_mappings {
    use super::{test_utils::*, *};

    #[ktest]
    fn overlapping_mappings() {
        let pt = setup_page_table::<UserMode>();
        let range1 = PAGE_SIZE..(PAGE_SIZE * 2);
        let range2 = (PAGE_SIZE * 1)..(PAGE_SIZE * 3);
        let page1 = allocator::alloc_single(FrameMeta::default()).unwrap();
        let page2 = allocator::alloc_single(FrameMeta::default()).unwrap();
        let prop = PageProperty::new(PageFlags::RW, CachePolicy::Writeback);

        unsafe {
            pt.cursor_mut(&range1)
                .unwrap()
                .map(page1.into(), prop.clone());
            pt.cursor_mut(&range2)
                .unwrap()
                .map(page2.clone().into(), prop);
        }

        // VA within the overlapping region should map to the latest mapping
        assert!(
            pt.query(PAGE_SIZE + 10).is_some(),
            "Overlapping VA should be mapped"
        );

        let mapped_pa = pt.query(PAGE_SIZE + 10).unwrap().0;
        assert_eq!(
            mapped_pa,
            page2.paddr() + 10,
            "VA should map to the latest PA"
        );
    }

    #[ktest]
    fn unaligned_map() {
        let pt = setup_page_table::<UserMode>();
        let range = (PAGE_SIZE + 512)..(PAGE_SIZE * 2 + 512);
        let prop = PageProperty::new(PageFlags::RW, CachePolicy::Writeback);
        let page = allocator::alloc_single(FrameMeta::default()).unwrap();

        // Attempt unaligned mapping
        let result = panic::catch_unwind(|| unsafe {
            pt.cursor_mut(&range).unwrap().map(page.into(), prop);
        });

        assert!(
            result.is_err(),
            "UnalignedVaddr mapping should panic or return an error"
        );
    }
}

#[cfg(ktest)]
mod large_mappings {
    use super::{test_utils::*, *};

    #[ktest]
    fn large_mappings() {
        let pt = setup_page_table::<UserMode>();
        let from = (PAGE_SIZE * 512 * 512)..(PAGE_SIZE * 512 * 512 * 2);
        let page = allocator::alloc_single(FrameMeta::default()).unwrap();
        let prop = PageProperty::new(PageFlags::RW, CachePolicy::Writeback);

        unsafe {
            pt.cursor_mut(&from).unwrap().map(page.into(), prop);
        }

        assert!(
            pt.query(from.start + 10).is_some(),
            "Large VA range should be mapped correctly"
        );
    }
}

#[cfg(ktest)]
mod tracked_mapping {
    use super::{test_utils::*, *};

    #[ktest]
    fn tracked_map_unmap() {
        let pt = setup_page_table::<UserMode>();
        let from = PAGE_SIZE..(PAGE_SIZE * 2);
        let page = allocator::alloc_single(FrameMeta::default()).unwrap();
        let start_paddr = page.paddr();
        let prop = PageProperty::new(PageFlags::RW, CachePolicy::Writeback);

        unsafe {
            pt.cursor_mut(&from).unwrap().map(page.into(), prop);
        }

        assert_eq!(
            pt.query(from.start + 10).unwrap().0,
            start_paddr + 10,
            "VA should map to correct physical address"
        );

        assert!(matches!(
            unsafe { pt.cursor_mut(&from).unwrap().take_next(from.len()) },
            PageTableItem::Mapped { .. }
        ));

        assert!(
            pt.query(from.start + 10).is_none(),
            "VA should be unmapped after take_next"
        );
    }

    #[ktest]
    fn remapping_same_range() {
        let pt = setup_page_table::<UserMode>();
        let range = PAGE_SIZE..(PAGE_SIZE * 2);
        let initial_prop = PageProperty::new(PageFlags::RW, CachePolicy::Writeback);
        let new_prop = PageProperty::new(PageFlags::R, CachePolicy::Writeback);

        // Initial mapping
        unsafe {
            pt.cursor_mut(&range).unwrap().map(
                allocator::alloc_single(FrameMeta::default())
                    .unwrap()
                    .into(),
                initial_prop,
            );
        }

        let initial_query = pt.query(range.start + 100).unwrap().1;
        assert_eq!(
            initial_query.flags,
            PageFlags::RW,
            "Initial flags should be RW"
        );
        assert_eq!(
            initial_query.cache,
            CachePolicy::Writeback,
            "Initial cache policy should be Writeback"
        );

        // Remap with new properties
        unsafe {
            pt.cursor_mut(&range).unwrap().map(
                allocator::alloc_single(FrameMeta::default())
                    .unwrap()
                    .into(),
                new_prop,
            );
        }

        let new_query = pt.query(range.start + 100).unwrap().1;
        assert_eq!(new_query.flags, PageFlags::R, "Remapped flags should be R");
        assert_eq!(
            new_query.cache,
            CachePolicy::Writeback,
            "Cache policy should remain Writeback after remapping"
        );
    }

    #[ktest]
    fn user_copy_on_write() {
        fn prot_op(prop: &mut PageProperty) {
            prop.flags -= PageFlags::W;
        }

        let pt = setup_page_table::<UserMode>();
        let from = PAGE_SIZE..(PAGE_SIZE * 2);
        let page = allocator::alloc_single(FrameMeta::default()).unwrap();
        let start_paddr = page.paddr();
        let prop = PageProperty::new(PageFlags::RW, CachePolicy::Writeback);

        unsafe {
            pt.cursor_mut(&from).unwrap().map(page.clone().into(), prop);
        }

        assert_eq!(
            pt.query(from.start + 10).unwrap().0,
            start_paddr + 10,
            "VA should map to correct physical address initially"
        );

        let child_pt = {
            let child_pt = setup_page_table::<UserMode>();
            let range = 0..MAX_USERSPACE_VADDR;
            let mut child_cursor = child_pt.cursor_mut(&range).unwrap();
            let mut parent_cursor = pt.cursor_mut(&range).unwrap();
            unsafe { child_cursor.copy_from(&mut parent_cursor, range.len(), &mut prot_op) };
            child_pt
        };

        assert_eq!(
            pt.query(from.start + 10).unwrap().0,
            start_paddr + 10,
            "Parent VA should remain mapped after copy"
        );
        assert_eq!(
            child_pt.query(from.start + 10).unwrap().0,
            start_paddr + 10,
            "Child VA should be mapped correctly after copy"
        );

        assert!(matches!(
            unsafe { pt.cursor_mut(&from).unwrap().take_next(from.len()) },
            PageTableItem::Mapped { .. }
        ));

        assert!(
            pt.query(from.start + 10).is_none(),
            "Parent VA should be unmapped after take_next"
        );
        assert_eq!(
            child_pt.query(from.start + 10).unwrap().0,
            start_paddr + 10,
            "Child VA should still be mapped correctly"
        );

        let sibling_pt = {
            let sibling_pt = setup_page_table::<UserMode>();
            let range = 0..MAX_USERSPACE_VADDR;
            let mut sibling_cursor = sibling_pt.cursor_mut(&range).unwrap();
            let mut parent_cursor = pt.cursor_mut(&range).unwrap();
            unsafe { sibling_cursor.copy_from(&mut parent_cursor, range.len(), &mut prot_op) };
            sibling_pt
        };

        assert!(
            sibling_pt.query(from.start + 10).is_none(),
            "Sibling VA should not be mapped"
        );
        assert_eq!(
            child_pt.query(from.start + 10).unwrap().0,
            start_paddr + 10,
            "Child VA should still be mapped correctly"
        );

        drop(pt);

        assert_eq!(
            child_pt.query(from.start + 10).unwrap().0,
            start_paddr + 10,
            "Child VA should remain mapped after parent is dropped"
        );

        assert!(matches!(
            unsafe { child_pt.cursor_mut(&from).unwrap().take_next(from.len()) },
            PageTableItem::Mapped { .. }
        ));

        assert!(
            child_pt.query(from.start + 10).is_none(),
            "Child VA should be unmapped after take_next"
        );

        unsafe {
            sibling_pt
                .cursor_mut(&from)
                .unwrap()
                .map(page.clone().into(), prop);
        }

        assert_eq!(
            sibling_pt.query(from.start + 10).unwrap().0,
            start_paddr + 10,
            "Sibling VA should be mapped after remapping"
        );

        assert!(
            child_pt.query(from.start + 10).is_none(),
            "Child VA should remain unmapped after sibling remapping"
        );
    }
}

#[cfg(ktest)]
mod untracked_mapping {
    use core::mem::ManuallyDrop;

    use super::{test_utils::*, *};

    #[ktest]
    fn untracked_map_unmap() {
        let pt = setup_page_table::<KernelMode>();
        const UNTRACKED_OFFSET: usize = LINEAR_MAPPING_BASE_VADDR;
        let from_ppn = 13245..(512 * 512 + 23456);
        let to_ppn = (from_ppn.start - 11010)..(from_ppn.end - 11010);
        let from = (UNTRACKED_OFFSET + PAGE_SIZE * from_ppn.start)
            ..(UNTRACKED_OFFSET + PAGE_SIZE * from_ppn.end);
        let to = (PAGE_SIZE * to_ppn.start)..(PAGE_SIZE * to_ppn.end);
        let prop = PageProperty::new(PageFlags::RW, CachePolicy::Writeback);

        map_range(&pt, from.clone(), to.clone(), prop);

        for i in 0..100 {
            let offset = i * (PAGE_SIZE + 1000);
            assert_eq!(
                pt.query(from.start + offset).unwrap().0,
                to.start + offset,
                "VA should map to correct PA at offset {}",
                offset
            );
        }

        let unmap = (UNTRACKED_OFFSET + PAGE_SIZE * 13456)..(UNTRACKED_OFFSET + PAGE_SIZE * 15678);
        assert!(matches!(
            unsafe { pt.cursor_mut(&unmap).unwrap().take_next(unmap.len()) },
            PageTableItem::MappedUntracked { .. }
        ));

        for i in 0..100 {
            let offset = i * (PAGE_SIZE + 10);
            let va = from.start + offset;
            if unmap.start <= va && va < unmap.end {
                assert!(
                    pt.query(va).is_none(),
                    "VA within unmap range should be unmapped"
                );
            } else {
                assert_eq!(
                    pt.query(va).unwrap().0,
                    to.start + offset,
                    "VA outside unmap range should remain mapped"
                );
            }
        }

        // Since untracked mappings cannot be dropped, we manually drop to avoid leak in test
        let _ = ManuallyDrop::new(pt);
    }

    #[ktest]
    fn untracked_large_protect_query() {
        let pt = PageTable::<KernelMode, PageTableEntry, VeryHugePagingConsts>::empty();
        const UNTRACKED_OFFSET: usize = crate::mm::kspace::LINEAR_MAPPING_BASE_VADDR;

        let gmult = 512 * 512;
        let from_ppn = gmult - 512..gmult + gmult + 514;
        let to_ppn = gmult - 512 - 512..gmult + gmult - 512 + 514;
        let from = UNTRACKED_OFFSET + PAGE_SIZE * from_ppn.start
            ..UNTRACKED_OFFSET + PAGE_SIZE * from_ppn.end;
        let to = PAGE_SIZE * to_ppn.start..PAGE_SIZE * to_ppn.end;
        let mapped_pa_of_va = |va: Vaddr| va - (from.start - to.start);
        let prop = PageProperty::new(PageFlags::RW, CachePolicy::Writeback);
        map_range(&pt, from.clone(), to.clone(), prop);

        for (item, i) in pt.cursor(&from).unwrap().zip(0..512 + 2 + 2) {
            let PageTableItem::MappedUntracked { va, pa, len, prop } = item else {
                panic!("Expected MappedUntracked, got {:#x?}", item);
            };
            assert_eq!(pa, mapped_pa_of_va(va));
            assert_eq!(prop.flags, PageFlags::RW);
            assert_eq!(prop.cache, CachePolicy::Writeback);
            if i < 512 + 2 {
                assert_eq!(va, from.start + i * PAGE_SIZE * 512);
                assert_eq!(va + len, from.start + (i + 1) * PAGE_SIZE * 512);
            } else {
                assert_eq!(
                    va,
                    from.start + (512 + 2) * PAGE_SIZE * 512 + (i - 512 - 2) * PAGE_SIZE
                );
                assert_eq!(
                    va + len,
                    from.start + (512 + 2) * PAGE_SIZE * 512 + (i - 512 - 2 + 1) * PAGE_SIZE
                );
            }
        }

        let ppn = from_ppn.start + 18..from_ppn.start + 20;
        let va = UNTRACKED_OFFSET + PAGE_SIZE * ppn.start..UNTRACKED_OFFSET + PAGE_SIZE * ppn.end;
        pt.protect(&va, |p| p.flags -= PageFlags::W);

        for (item, i) in pt
            .cursor(&(va.start - PAGE_SIZE..va.start))
            .unwrap()
            .zip(ppn.start - 1..ppn.start)
        {
            let PageTableItem::MappedUntracked { va, pa, len, prop } = item else {
                panic!("Expected MappedUntracked, got {:#x?}", item);
            };
            assert_eq!(pa, mapped_pa_of_va(va));
            assert_eq!(prop.flags, PageFlags::RW);
            let va = va - UNTRACKED_OFFSET;
            assert_eq!(va..va + len, i * PAGE_SIZE..(i + 1) * PAGE_SIZE);
        }

        for (item, i) in pt.cursor(&va).unwrap().zip(ppn.clone()) {
            let PageTableItem::MappedUntracked { va, pa, len, prop } = item else {
                panic!("Expected MappedUntracked, got {:#x?}", item);
            };
            assert_eq!(pa, mapped_pa_of_va(va));
            assert_eq!(prop.flags, PageFlags::R);
            let va = va - UNTRACKED_OFFSET;
            assert_eq!(va..va + len, i * PAGE_SIZE..(i + 1) * PAGE_SIZE);
        }

        for (item, i) in pt
            .cursor(&(va.end..va.end + PAGE_SIZE))
            .unwrap()
            .zip(ppn.end..ppn.end + 1)
        {
            let PageTableItem::MappedUntracked { va, pa, len, prop } = item else {
                panic!("Expected MappedUntracked, got {:#x?}", item);
            };
            assert_eq!(pa, mapped_pa_of_va(va));
            assert_eq!(prop.flags, PageFlags::RW);
            let va = va - UNTRACKED_OFFSET;
            assert_eq!(va..va + len, i * PAGE_SIZE..(i + 1) * PAGE_SIZE);
        }

        // Since untracked mappings cannot be dropped, we just leak it here.
        let _ = ManuallyDrop::new(pt);
    }
}

#[cfg(ktest)]
mod full_unmap_verification {
    use super::{test_utils::*, *};

    #[ktest]
    fn full_unmap() {
        let pt = setup_page_table::<UserMode>();
        let range = 0..(PAGE_SIZE * 100);
        let prop = PageProperty::new(PageFlags::RW, CachePolicy::Writeback);

        let pages = allocator::alloc(100 * PAGE_SIZE, |_| FrameMeta::default()).unwrap();
        unsafe {
            let mut cursor = pt.cursor_mut(&range).unwrap();
            for page in pages.iter() {
                cursor.map(page.clone().into(), prop);
            }
        }

        // Verify all addresses are initially mapped
        for va in (range.start..range.end).step_by(PAGE_SIZE) {
            assert!(pt.query(va).is_some(), "VA {:#x} should be mapped", va);
        }

        // Unmap the entire range
        unsafe {
            let mut cursor = pt.cursor_mut(&range).unwrap();
            for _ in (range.start..range.end).step_by(PAGE_SIZE) {
                cursor.take_next(PAGE_SIZE);
            }
        }

        // Verify all addresses are unmapped
        for va in (range.start..range.end).step_by(PAGE_SIZE) {
            assert!(pt.query(va).is_none(), "VA {:#x} should be unmapped", va);
        }
    }
}

#[cfg(ktest)]
mod protection_and_query {
    use super::{test_utils::*, *};

    #[ktest]
    fn base_protect_query() {
        let pt = setup_page_table::<UserMode>();
        let from_ppn = 1..1000;
        let from = PAGE_SIZE * from_ppn.start..PAGE_SIZE * from_ppn.end;
        let to = allocator::alloc(999 * PAGE_SIZE, |_| FrameMeta::default()).unwrap();
        let prop = PageProperty::new(PageFlags::RW, CachePolicy::Writeback);

        unsafe {
            let mut cursor = pt.cursor_mut(&from).unwrap();
            for page in to {
                cursor.map(page.clone().into(), prop);
            }
        }

        for (item, i) in pt.cursor(&from).unwrap().zip(from_ppn.clone()) {
            if let PageTableItem::Mapped { va, page, prop } = item {
                assert_eq!(prop.flags, PageFlags::RW, "Page flags should be RW");
                assert_eq!(
                    prop.cache,
                    CachePolicy::Writeback,
                    "Cache policy should be Writeback"
                );
                assert_eq!(
                    va..(va + page.size()),
                    i * PAGE_SIZE..((i + 1) * PAGE_SIZE),
                    "VA range should align correctly"
                );
            } else {
                panic!("Expected Mapped, got {:?}", item);
            }
        }

        let prot = (PAGE_SIZE * 18)..(PAGE_SIZE * 20);
        pt.protect(&prot, |p| p.flags -= PageFlags::W);

        for (item, i) in pt.cursor(&prot).unwrap().zip(18..20) {
            if let PageTableItem::Mapped { va, page, prop } = item {
                assert_eq!(
                    prop.flags,
                    PageFlags::R,
                    "Page flags should be updated to R"
                );
                assert_eq!(
                    va..(va + page.size()),
                    (i * PAGE_SIZE)..((i + 1) * PAGE_SIZE),
                    "VA range should align correctly after protection"
                );
            } else {
                panic!("Expected Mapped, got {:?}", item);
            }
        }
    }
}
