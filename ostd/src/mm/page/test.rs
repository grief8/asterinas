// SPDX-License-Identifier: MPL-2.0
use core::sync::atomic::Ordering;

use meta::impl_page_meta;

use super::{allocator::PAGE_ALLOCATOR, *};
use crate::{
    mm::{
        frame::FrameMeta,
        page::{allocator, Page},
    },
    prelude::*,
};

// Mock metadata struct for testing
#[derive(Debug, Default)]
struct MockPageMeta {
    value: u32,
}
impl_page_meta!(MockPageMeta);

// Basic metadata tests
#[cfg(ktest)]
mod metadata_tests {
    use super::*;

    #[ktest]
    fn test_metadata_size_constraints() {
        assert!(core::mem::size_of::<MockPageMeta>() <= PAGE_METADATA_MAX_SIZE);
        assert!(core::mem::align_of::<MockPageMeta>() <= PAGE_METADATA_MAX_ALIGN);
    }

    #[ktest]
    fn test_meta_slot_layout() {
        const META_SLOT_SIZE: usize = 64;
        assert_eq!(core::mem::size_of::<MetaSlot>(), META_SLOT_SIZE);
        assert_eq!(PAGE_SIZE % META_SLOT_SIZE, 0);
    }

    #[ktest]
    fn test_metadata_mapping() {
        let test_paddr = 0x1000;
        let meta_vaddr = mapping::page_to_meta::<PagingConsts>(test_paddr);
        let mapped_paddr = mapping::meta_to_page::<PagingConsts>(meta_vaddr);
        assert_eq!(test_paddr, mapped_paddr);
    }
}

// Page allocation and management tests
#[cfg(ktest)]
mod page_tests {
    use super::*;

    #[ktest]
    fn test_page_allocation() {
        let meta = MockPageMeta { value: 42 };
        let page = allocator::alloc_single(meta).expect("Failed to allocate single page");
        assert_eq!(page.meta().value, 42);
        assert_eq!(page.reference_count(), 1);
        assert_eq!(page.size(), PAGE_SIZE);
        assert_eq!(page.level(), 1);
    }

    #[ktest]
    fn test_page_from_unused() {
        // Since we cannot specify paddr directly through the allocator,
        // we'll allocate a page and verify its properties.
        let metadata = MockPageMeta { value: 84 };
        let page = allocator::alloc_single(metadata).expect("Failed to allocate single page");
        assert_eq!(page.meta().value, 84);
    }

    #[ktest]
    fn test_page_clone() {
        let meta = MockPageMeta { value: 210 };
        let page1 = allocator::alloc_single(meta).expect("Failed to allocate single page");
        let page2 = page1.clone();
        assert_eq!(page1.paddr(), page2.paddr());
        assert_eq!(page1.meta().value, page2.meta().value);
        assert_eq!(page1.reference_count(), 2);
        assert_eq!(page2.reference_count(), 2);
    }

    #[ktest]
    fn test_page_drop() {
        let metadata = MockPageMeta { value: 252 };
        let page = allocator::alloc_single(metadata).expect("Failed to allocate single page");
        let ref_count_before = page.reference_count();
        let paddr_before = page.paddr();
        assert_eq!(ref_count_before, 1);
        drop(page);
        // Assuming reference counting is handled correctly,
        // creating a new page (which may reuse the same paddr) should have reference_count = 1
        let new_page = allocator::alloc_single(MockPageMeta { value: 300 })
            .expect("Failed to allocate single page");
        assert_eq!(new_page.paddr(), paddr_before);
        assert_eq!(new_page.reference_count(), 1);
        assert_eq!(new_page.meta().value, 300);
    }
}

// DynPage tests
#[cfg(ktest)]
mod dyn_page_tests {
    use super::*;

    #[ktest]
    fn test_dyn_page_conversion() {
        let meta = MockPageMeta { value: 42 };
        let typed_page = allocator::alloc_single(meta).expect("Failed to allocate single page");
        let dyn_page: DynPage = typed_page.clone().into();
        // alloc_single: 1, typed_page.clone: 2
        assert_eq!(typed_page.reference_count(), 2);
        // Try converting back
        let typed_page_result: core::result::Result<Page<MockPageMeta>, DynPage> =
            Page::try_from(dyn_page.clone());
        assert!(typed_page_result.is_ok());
        let restored_page = typed_page_result.unwrap();
        assert_eq!(restored_page.meta().value, 42);
    }

    #[ktest]
    fn test_dyn_page_meta() {
        let meta = MockPageMeta { value: 168 };
        let page = allocator::alloc_single(meta).expect("Failed to allocate single page");
        let dyn_page = DynPage::from(page);
        let meta_ref = dyn_page.meta();
        assert_eq!(meta_ref.downcast_ref::<MockPageMeta>().unwrap().value, 168);
    }

    #[ktest]
    fn test_dyn_page_clone() {
        let meta = MockPageMeta { value: 210 };
        let page = allocator::alloc_single(meta).expect("Failed to allocate single page");
        let dyn_page = DynPage::from(page.clone());
        let cloned_dyn_page = dyn_page.clone();
        assert_eq!(cloned_dyn_page.paddr(), dyn_page.paddr());
        // alloc_single: 1, page.clone: 2, dyn_page.clone: 3
        assert_eq!(page.reference_count(), 3);
    }

    #[ktest]
    fn test_dyn_page_drop() {
        let meta = MockPageMeta { value: 252 };
        let page = allocator::alloc_single(meta).expect("Failed to allocate single page");
        let dyn_page = DynPage::from(page.clone());
        let ref_count_before = page.reference_count();
        // Depending on implementation, check if ref_count is decremented
        drop(dyn_page);
        let ref_count_after = page.reference_count();
        assert_eq!(ref_count_after, ref_count_before - 1);
    }

    #[ktest]
    fn test_dyn_page_wrong_type_conversion() {
        let meta = MockPageMeta { value: 42 };
        let page = allocator::alloc_single(meta).expect("Failed to allocate single page");
        let dyn_page: DynPage = DynPage::from(page);
        // Try converting to wrong type
        let wrong_conversion: core::result::Result<Page<FrameMeta>, DynPage> =
            Page::try_from(dyn_page.clone());
        assert!(wrong_conversion.is_err());
        assert_eq!(wrong_conversion.unwrap_err().paddr(), dyn_page.paddr());
    }
}

// TryFrom<DynPage> for Page tests
#[cfg(ktest)]
mod try_from_dyn_page_tests {
    use super::*;

    #[ktest]
    fn test_try_from_dyn_page_success() {
        let meta = MockPageMeta { value: 42 };
        let page = allocator::alloc_single(meta).expect("Failed to allocate single page");
        let dyn_page = DynPage::from(page);
        let result = Page::<MockPageMeta>::try_from(dyn_page.clone());
        assert!(result.is_ok());
        let restored_page = result.unwrap();
        assert_eq!(restored_page.paddr(), dyn_page.paddr());
        assert_eq!(restored_page.meta().value, 42);
    }

    #[ktest]
    fn test_try_from_dyn_page_failure() {
        let meta = MockPageMeta { value: 42 };
        let page = allocator::alloc_single(meta).expect("Failed to allocate single page");
        let dyn_page = DynPage::from(page);
        let result = Page::<FrameMeta>::try_from(dyn_page.clone());
        assert!(result.is_err());
        let restored_dyn_page = result.unwrap_err();
        assert_eq!(restored_dyn_page.paddr(), dyn_page.paddr());
    }
}

// Reference count tests
#[cfg(ktest)]
mod inc_page_ref_count_tests {
    use super::*;

    #[ktest]
    fn test_inc_page_ref_count() {
        let meta = MockPageMeta { value: 42 };
        let page = allocator::alloc_single(meta).expect("Failed to allocate single page");
        let ref_count_before = page.reference_count();
        unsafe { inc_page_ref_count(page.paddr()) };
        let ref_count_after = page.reference_count();
        assert_eq!(ref_count_after, ref_count_before + 1);
    }
}

// Contiguous pages tests
#[cfg(ktest)]
mod cont_pages_tests {
    use super::*;

    #[ktest]
    fn test_cont_pages_creation() {
        let range = 512 * PAGE_SIZE..1024 * PAGE_SIZE;
        let pages = allocator::alloc_contiguous(range.len(), |_| MockPageMeta { value: 42 })
            .expect("Failed to allocate contiguous pages");
        assert_eq!(pages.nbytes(), range.len());
        assert_eq!(pages.start_paddr(), range.start);
        assert_eq!(pages.end_paddr(), range.end);
    }

    #[ktest]
    fn test_cont_pages_split() {
        let total_pages = 2;
        let pages =
            allocator::alloc_contiguous(total_pages * PAGE_SIZE, |_| MockPageMeta { value: 42 })
                .expect("Failed to allocate contiguous pages");
        let (first, second) = pages.split(PAGE_SIZE);
        assert_eq!(first.nbytes(), PAGE_SIZE);
        assert_eq!(second.nbytes(), PAGE_SIZE);
    }

    #[ktest]
    fn test_cont_pages_slice() {
        let total_pages = 3;
        let pages =
            allocator::alloc_contiguous(total_pages * PAGE_SIZE, |_| MockPageMeta { value: 42 })
                .expect("Failed to allocate contiguous pages");
        let slice = pages.slice(&(PAGE_SIZE..PAGE_SIZE * 2));
        assert_eq!(slice.nbytes(), PAGE_SIZE);
        assert_eq!(slice.start_paddr(), pages.start_paddr() + PAGE_SIZE);
    }

    #[ktest]
    fn test_cont_pages_iteration() {
        let total_pages = 2;
        let pages =
            allocator::alloc_contiguous(total_pages * PAGE_SIZE, |_| MockPageMeta { value: 42 })
                .expect("Failed to allocate contiguous pages");
        let mut count = 0;
        for page in pages {
            assert_eq!(page.meta().value, 42);
            count += 1;
        }
        assert_eq!(count, total_pages);
    }

    #[ktest]
    #[should_panic]
    fn test_invalid_cont_pages_split() {
        let total_pages = 2;
        let pages =
            allocator::alloc_contiguous(total_pages * PAGE_SIZE, |_| MockPageMeta { value: 42 })
                .expect("Failed to allocate contiguous pages");
        // Attempt to split at zero, which should panic
        pages.split(0);
    }
}

// Page allocator tests
#[cfg(ktest)]
mod allocator_tests {
    use super::*;

    #[ktest]
    fn test_single_page_alloc() {
        let page = allocator::alloc_single(MockPageMeta { value: 42 });
        assert!(page.is_some());
        let page = page.unwrap();
        assert_eq!(page.meta().value, 42);
    }

    #[ktest]
    fn test_contiguous_pages_alloc() {
        let pages = allocator::alloc_contiguous(PAGE_SIZE * 2, |_| MockPageMeta { value: 42 });
        assert!(pages.is_some());
        let pages = pages.unwrap();
        assert_eq!(pages.nbytes(), PAGE_SIZE * 2);
    }

    #[ktest]
    fn test_multiple_pages_alloc() {
        let pages = allocator::alloc(PAGE_SIZE * 2, |_| MockPageMeta { value: 42 });
        assert!(pages.is_some());
        let pages = pages.unwrap();
        assert_eq!(pages.len(), 2);
        for page in pages {
            assert_eq!(page.meta().value, 42);
        }
    }
}

// Memory accounting tests
#[cfg(ktest)]
mod memory_accounting_tests {
    use super::*;

    #[ktest]
    fn test_allocator_counting() {
        let initial_available = PAGE_ALLOCATOR.get().unwrap().lock().mem_available();
        let page = allocator::alloc_single(MockPageMeta { value: 42 }).unwrap();
        let after_alloc = PAGE_ALLOCATOR.get().unwrap().lock().mem_available();
        assert_eq!(initial_available - after_alloc, PAGE_SIZE);
        drop(page);
        let after_free = PAGE_ALLOCATOR.get().unwrap().lock().mem_available();
        assert_eq!(after_free, initial_available);
    }

    #[ktest]
    fn test_max_paddr_tracking() {
        let test_max = 0x1_0000_0000;
        MAX_PADDR.store(test_max, Ordering::Relaxed);
        assert_eq!(MAX_PADDR.load(Ordering::Relaxed), test_max);
    }
}
