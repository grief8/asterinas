// test.rs

use core::alloc::Layout;

use super::{Heap, *};
use crate::{mm::page_allocator::PAGE_SIZE, prelude::*};

#[ktest]
fn test_heap_initialization() {
    unsafe {
        // Initialize the heap allocator
        init();

        // Verify that the heap allocator is initialized
        assert!(HEAP_ALLOCATOR.heap.get().is_some());
    }
}

#[ktest]
fn test_locked_heap_with_rescue_new() {
    let locked_heap = LockedHeapWithRescue::new();
    assert!(locked_heap.heap.get().is_none());
}

#[ktest]
fn test_locked_heap_with_rescue_alloc() {
    unsafe {
        init();

        // Allocate a small chunk of memory
        let layout = Layout::from_size_align(16, 8).unwrap();
        let ptr = HEAP_ALLOCATOR.alloc(layout);

        // Verify that the allocation was successful
        assert!(!ptr.is_null());

        // Deallocate the memory
        HEAP_ALLOCATOR.dealloc(ptr, layout);
    }
}

#[ktest]
fn test_locked_heap_with_rescue_dealloc() {
    unsafe {
        init();

        // Allocate a small chunk of memory
        let layout = Layout::from_size_align(16, 8).unwrap();
        let ptr = HEAP_ALLOCATOR.alloc(layout);

        // Deallocate the memory
        HEAP_ALLOCATOR.dealloc(ptr, layout);

        // Verify that the deallocation was successful by trying to allocate again
        let ptr2 = HEAP_ALLOCATOR.alloc(layout);
        assert!(!ptr2.is_null());

        // Clean up
        HEAP_ALLOCATOR.dealloc(ptr2, layout);
    }
}

#[ktest]
fn test_locked_heap_with_rescue_rescue() {
    unsafe {
        init();

        // Allocate a large chunk of memory to trigger the rescue mechanism
        let layout = Layout::from_size_align(PAGE_SIZE * 1024, PAGE_SIZE).unwrap();
        let ptr = HEAP_ALLOCATOR.alloc(layout);

        // Verify that the allocation was successful
        assert!(!ptr.is_null());

        // Deallocate the memory
        HEAP_ALLOCATOR.dealloc(ptr, layout);
    }
}

#[ktest]
fn heap_stat() {
    unsafe {
        let mut buffer = InitHeapSpace([0u8; PAGE_SIZE * 8]);
        let mut heap = Heap::new(buffer.0.as_mut_ptr() as usize, PAGE_SIZE * 8);

        let layout: Layout = Layout::from_size_align(16, 8).unwrap();
        let size = heap.usable_size(layout);
        assert_eq!(size.0, 16);

        let total_bytes = heap.total_bytes();
        assert_eq!(total_bytes, PAGE_SIZE * 8);

        let used_bytes = heap.used_bytes();
        assert_eq!(used_bytes, 0);

        let available_bytes = heap.available_bytes();
        assert_eq!(available_bytes, PAGE_SIZE * 8);
    }
}
