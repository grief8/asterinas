// test.rs

use super::*;
use core::alloc::Layout;
use crate::prelude::*;

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

// #[ktest] 
// fn test_locked_heap_with_rescue_init() {
//     let locked_heap = LockedHeapWithRescue::new();
//     const HEAP_SIZE: usize = PAGE_SIZE * 256;
//     let mut heap_space = [0u8; HEAP_SIZE];

//     unsafe {
//         locked_heap.init(heap_space.as_mut_ptr(), HEAP_SIZE);
//         assert!(locked_heap.heap.get().is_some());
//     }
// }

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

// #[ktest]
// fn alloc_stat() {
//     unsafe {
//         init(); 
//         HEAP_ALLOCATOR.stat();
//     }
// }

// #[ktest]
// fn used_bytes() {
//     unsafe {
//         init();

        
//     }
// }
// #[ktest] 
// fn test_locked_heap_with_rescue_rescue_if_low_memory() {
//     unsafe {
//         init();

//         // Allocate memory until the heap is nearly exhausted
//         let layout = Layout::from_size_align(PAGE_SIZE, PAGE_SIZE).unwrap();
//         let mut ptrs = Vec::new();
//         loop {
//             let ptr = HEAP_ALLOCATOR.alloc(layout);
//             if ptr.is_null() {
//                 break;
//             }
//             ptrs.push(ptr);
//         }

//         // Verify that the rescue mechanism was triggered
//         assert!(ptrs.len() > 0);

//         // Deallocate all allocated memory
//         for ptr in ptrs {
//             HEAP_ALLOCATOR.dealloc(ptr, layout);
//         }
//     }
// }

// #[ktest] 
// fn test_locked_heap_with_rescue_add_to_heap() {
//     unsafe {
//         init();

//         // Allocate additional memory and add it to the heap
//         const ADDITIONAL_HEAP_SIZE: usize = PAGE_SIZE * 128;
//         let additional_heap_space = [0u8; ADDITIONAL_HEAP_SIZE];
//         HEAP_ALLOCATOR.add_to_heap(additional_heap_space.as_ptr() as usize, ADDITIONAL_HEAP_SIZE);

//         // Allocate a small chunk of memory from the additional heap space
//         let layout = Layout::from_size_align(16, 8).unwrap();
//         let ptr = HEAP_ALLOCATOR.alloc(layout);

//         // Verify that the allocation was successful
//         assert!(!ptr.is_null());

//         // Deallocate the memory
//         HEAP_ALLOCATOR.dealloc(ptr, layout);
//     }
// }