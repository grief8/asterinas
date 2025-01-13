// SPDX-License-Identifier: MPL-2.0

use super::{allocator::FrameAllocOptions, *};
use crate::{impl_frame_meta_for, impl_untyped_frame_meta_for, mm::USegment, prelude::*};

// Typed mock metadata struct for testing
#[derive(Debug, Default)]
struct MockFrameMeta {
    value: u32,
}
impl_frame_meta_for!(MockFrameMeta);

/// Untyped mock metadata struct for testing
#[derive(Debug, Default)]
struct MockUFrameMeta {
    value: u32,
}
impl_untyped_frame_meta_for!(MockUFrameMeta);

// Basic metadata tests
mod metadata {
    use super::*;

    #[ktest]
    fn metadata_size_constraints() {
        assert!(core::mem::size_of::<MockFrameMeta>() <= FRAME_METADATA_MAX_SIZE);
        assert!(core::mem::align_of::<MockFrameMeta>() <= FRAME_METADATA_MAX_ALIGN);
    }

    #[ktest]
    fn meta_slot_layout() {
        const META_SLOT_SIZE: usize = 64;
        assert_eq!(core::mem::size_of::<MetaSlot>(), META_SLOT_SIZE);
        assert_eq!(PAGE_SIZE % META_SLOT_SIZE, 0);
    }

    #[ktest]
    fn metadata_mapping() {
        let test_paddr = 0x1000;
        let meta_vaddr = mapping::frame_to_meta::<PagingConsts>(test_paddr);
        let mapped_paddr = mapping::meta_to_frame::<PagingConsts>(meta_vaddr);
        assert_eq!(test_paddr, mapped_paddr);
    }
}

// Frame allocation and management tests
mod frame {
    use super::*;

    #[ktest]
    fn frame_allocation() {
        let meta = MockFrameMeta { value: 42 };
        let frame = FrameAllocOptions::new()
            .alloc_frame_with(meta)
            .expect("Failed to allocate single frame");
        assert_eq!(frame.meta().value, 42);
        assert_eq!(frame.reference_count(), 1);
        assert_eq!(frame.size(), PAGE_SIZE);
        assert_eq!(frame.level(), 1);
    }

    #[ktest]
    fn frame_clone() {
        let meta = MockFrameMeta { value: 42 };
        let frame1 = FrameAllocOptions::new()
            .alloc_frame_with(meta)
            .expect("Failed to allocate single frame");
        let frame2 = frame1.clone();
        assert_eq!(frame1.start_paddr(), frame2.start_paddr());
        assert_eq!(frame1.meta().value, frame2.meta().value);
        assert_eq!(frame1.reference_count(), 2);
        assert_eq!(frame2.reference_count(), 2);
    }

    #[ktest]
    fn frame_drop() {
        let metadata = MockFrameMeta { value: 42 };
        let frame = FrameAllocOptions::new()
            .alloc_frame_with(metadata)
            .expect("Failed to allocate single frame");
        let ref_count_before = frame.reference_count();
        let paddr_before = frame.start_paddr();
        assert_eq!(ref_count_before, 1);
        drop(frame);
        let new_frame = FrameAllocOptions::new()
            .alloc_frame_with(MockFrameMeta { value: 42 })
            .expect("Failed to allocate single frame");
        assert_eq!(new_frame.start_paddr(), paddr_before);
        assert_eq!(new_frame.reference_count(), 1);
        assert_eq!(new_frame.meta().value, 42);
    }

    #[ktest]
    fn frame_to_uframe() {
        let frame = FrameAllocOptions::new()
            .alloc_frame_with(MockUFrameMeta { value: 42 })
            .unwrap();
        let uframe: UFrame = frame.into();
        assert_eq!(uframe.size(), PAGE_SIZE);
    }

    #[ktest]
    fn frame_conversions() {
        let frame = FrameAllocOptions::new()
            .alloc_frame_with(MockFrameMeta { value: 42 })
            .unwrap();
        let dyn_frame: Frame<dyn AnyFrameMeta> = frame.into();
        assert!(!dyn_frame.dyn_meta().is_untyped());
        let result: core::result::Result<Frame<MockFrameMeta>, _> = Frame::try_from(dyn_frame);
        assert!(result.is_ok());
        assert_eq!(result.unwrap().meta().value, 42);
    }
}

// Segment tests
mod segment {
    use super::*;
    use crate::Error;

    #[ktest]
    fn segment_creation() {
        let range = 512 * PAGE_SIZE..513 * PAGE_SIZE;
        let segment = FrameAllocOptions::new()
            .alloc_segment(range.len() / PAGE_SIZE)
            .expect("Failed to allocate segment");
        assert_eq!(segment.size(), range.len());
        // assert_eq!(segment.start_paddr(), range.start);
        // assert_eq!(segment.end_paddr(), range.end);
    }

    #[ktest]
    #[should_panic]
    fn max_segment_creation() {
        // Upstream FrameAllocator panics when attempting to allocate a segment with usize::MAX frames
        let max_frames = usize::MAX;
        let _ = FrameAllocOptions::new().alloc_segment(max_frames);
    }

    #[ktest]
    fn empty_segment_creation() {
        let result = FrameAllocOptions::new().alloc_segment(0);
        assert!(
            matches!(result, Err(Error::InvalidArgs)),
            "Expected `InvalidArgs` error when allocating a zero-sized segment"
        );
    }

    #[ktest]
    fn zeroed_segment_creation() {
        let segment = FrameAllocOptions::new()
            .zeroed(true)
            .alloc_segment(1)
            .expect("Failed to allocate segment");
        let mut reader = segment.reader();
        let mut buffer = [0; PAGE_SIZE];
        reader.read(&mut buffer.as_mut_slice().into());
        assert!(buffer.iter().all(|&x| x == 0));
    }

    #[ktest]
    fn segment_split() {
        let total_frames = 2;
        let segment = FrameAllocOptions::new()
            .alloc_segment(total_frames)
            .expect("Failed to allocate segment");
        let (first, second) = segment.split(PAGE_SIZE);
        assert_eq!(first.size(), PAGE_SIZE);
        assert_eq!(second.size(), PAGE_SIZE);
    }

    #[ktest]
    fn segment_slice() {
        let total_frames = 3;
        let segment = FrameAllocOptions::new()
            .alloc_segment(total_frames)
            .expect("Failed to allocate segment");
        let slice = segment.slice(&(PAGE_SIZE..PAGE_SIZE * 2));
        assert_eq!(slice.size(), PAGE_SIZE);
        assert_eq!(slice.start_paddr(), segment.start_paddr() + PAGE_SIZE);
    }

    #[ktest]
    fn segment_iteration() {
        let total_frames = 2;
        let segment = FrameAllocOptions::new()
            .alloc_segment_with(total_frames, |_| MockFrameMeta { value: 42 })
            .expect("Failed to allocate segment");
        let mut count = 0;
        for frame in segment {
            assert_eq!(frame.meta().value, 42);
            count += 1;
        }
        assert_eq!(count, total_frames);
    }

    #[ktest]
    #[should_panic]
    fn invalid_segment_split() {
        let total_frames = 2;
        let segment = FrameAllocOptions::new()
            .alloc_segment(total_frames)
            .expect("Failed to allocate segment");
        // Attempt to split at zero, which should panic
        segment.split(0);
    }

    #[ktest]
    fn segment_to_usegment() {
        let options = FrameAllocOptions::new();
        let segment = options.alloc_segment(1).unwrap();
        let dyn_segment: Segment<dyn AnyFrameMeta> = segment.clone().into();
        let result: core::result::Result<USegment, Segment<_>> = USegment::try_from(dyn_segment);
        assert!(result.is_ok());
        let usegment = result.unwrap();
        assert_eq!(usegment.size(), PAGE_SIZE);
        assert_eq!(usegment.start_paddr(), segment.start_paddr());
    }

    #[ktest]
    fn segment_to_segment() {
        let options = FrameAllocOptions::new();
        let segment = options
            .alloc_segment_with(1, |_| MockFrameMeta { value: 42 })
            .unwrap();
        let dyn_segment: Segment<dyn AnyFrameMeta> = segment.into();
        let result: core::result::Result<Segment<MockFrameMeta>, Segment<_>> =
            Segment::try_from(dyn_segment);
        assert!(result.is_ok());
        let segment = result.unwrap();
        assert_eq!(segment.size(), PAGE_SIZE);
        for frame in segment {
            assert_eq!(frame.meta().value, 42);
        }
    }

    #[ktest]
    fn frame_to_segment() {
        let frame = FrameAllocOptions::new()
            .alloc_frame_with(MockFrameMeta { value: 42 })
            .unwrap();
        let paddr = frame.start_paddr();
        let segment: Segment<MockFrameMeta> = frame.into();
        assert_eq!(segment.size(), PAGE_SIZE);
        assert_eq!(segment.start_paddr(), paddr);
        for frame in segment {
            assert_eq!(frame.meta().value, 42);
        }
    }

    #[ktest]
    fn segment_drop() {
        let options = FrameAllocOptions::new();
        let segment = options.alloc_segment(1).unwrap();
        let paddr_before = segment.start_paddr();
        drop(segment);
        let new_segment = options.alloc_segment(1).unwrap();
        assert_eq!(new_segment.start_paddr(), paddr_before);
    }
}

mod untyped {
    use super::*;
    use crate::mm::frame::untyped::FrameRef;

    #[ktest]
    fn untyped_frame_reader_writer() {
        let frame = FrameAllocOptions::new()
            .alloc_frame_with(())
            .expect("Failed to allocate frame");

        // Test reader
        let mut reader = frame.reader();
        assert_eq!(reader.remain(), PAGE_SIZE);

        // Test writer
        let mut writer = frame.writer();
        assert_eq!(writer.avail(), PAGE_SIZE);

        // Write some data to the frame
        let data = [0xAA; 128];
        writer.write(&mut data.as_slice().into());

        // Read back the data
        let mut buffer = [0; 128];
        reader.read(&mut buffer.as_mut_slice().into());
        assert_eq!(buffer, data);
    }

    #[ktest]
    fn untyped_segment_reader_writer() {
        let segment = FrameAllocOptions::new()
            .alloc_segment(2)
            .expect("Failed to allocate segment");

        // Test reader
        let mut reader = segment.reader();
        assert_eq!(reader.remain(), 2 * PAGE_SIZE);

        // Test writer
        let mut writer = segment.writer();
        assert_eq!(writer.avail(), 2 * PAGE_SIZE);

        // Write some data to the segment
        let data = [0xBB; 256];
        writer.write(&mut data.as_slice().into());

        // Read back the data
        let mut buffer = [0; 256];
        reader.read(&mut buffer.as_mut_slice().into());
        assert_eq!(buffer, data);
    }

    #[ktest]
    fn xarray_item_entry() {
        use xarray::ItemEntry;

        let init_val = 42;
        let frame = FrameAllocOptions::new()
            .alloc_frame_with(MockUFrameMeta { value: init_val })
            .expect("Failed to allocate frame");
        let ptr = frame.start_paddr();
        let uframe: UFrame = frame.into();

        let raw_ptr = ItemEntry::into_raw(uframe);
        let frame_from_raw: Frame<MockUFrameMeta> = unsafe { ItemEntry::from_raw(raw_ptr) };
        assert_eq!(frame_from_raw.start_paddr(), ptr);
        assert_eq!(frame_from_raw.start_paddr(), ptr);
        assert_eq!(frame_from_raw.meta().value, init_val);

        let frame_ref: FrameRef<MockUFrameMeta> = unsafe { Frame::raw_as_ref(raw_ptr) };
        assert_eq!(frame_ref.start_paddr(), ptr);
    }
}
