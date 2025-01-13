// SPDX-License-Identifier: MPL-2.0

use alloc::vec;

use crate::{
    mm::{dma::*, io::VmIo, FrameAllocOptions, HasPaddr, PAGE_SIZE},
    prelude::*,
};

#[ktest]
fn test_dma_coherent_map() {
    let segment = FrameAllocOptions::new()
        .alloc_segment_with(1, |_| ())
        .unwrap();
    let dma_coherent = DmaCoherent::map(segment.clone().into(), true).unwrap();
    assert_eq!(dma_coherent.paddr(), segment.start_paddr());
    assert_eq!(dma_coherent.nbytes(), PAGE_SIZE);
}

#[ktest]
fn test_dma_coherent_map_incoherent() {
    let segment = FrameAllocOptions::new()
        .alloc_segment_with(1, |_| ())
        .unwrap();
    let dma_coherent = DmaCoherent::map(segment.clone().into(), false).unwrap();
    assert_eq!(dma_coherent.paddr(), segment.start_paddr());
    assert_eq!(dma_coherent.nbytes(), PAGE_SIZE);
}

#[ktest]
fn test_dma_coherent_duplicate_map() {
    let segment = FrameAllocOptions::new()
        .alloc_segment_with(2, |_| ())
        .unwrap();
    let segment_child = segment.slice(&(0..PAGE_SIZE));
    let _dma_coherent_parent = DmaCoherent::map(segment.into(), false).unwrap();
    let dma_coherent_child = DmaCoherent::map(segment_child.into(), false);
    assert!(dma_coherent_child.is_err());
}

#[ktest]
fn test_dma_coherent_read_write() {
    let segment = FrameAllocOptions::new()
        .alloc_segment_with(2, |_| ())
        .unwrap();
    let dma_coherent = DmaCoherent::map(segment.into(), false).unwrap();

    let buf_write = vec![1u8; 2 * PAGE_SIZE];
    dma_coherent.write_bytes(0, &buf_write).unwrap();
    let mut buf_read = vec![0u8; 2 * PAGE_SIZE];
    dma_coherent.read_bytes(0, &mut buf_read).unwrap();
    assert_eq!(buf_write, buf_read);
}

#[ktest]
fn test_dma_coherent_reader_writer() {
    let segment = FrameAllocOptions::new()
        .alloc_segment_with(2, |_| ())
        .unwrap();
    let dma_coherent = DmaCoherent::map(segment.into(), false).unwrap();

    let buf_write = vec![1u8; PAGE_SIZE];
    let mut writer = dma_coherent.writer();
    writer.write(&mut buf_write.as_slice().into());
    writer.write(&mut buf_write.as_slice().into());

    let mut buf_read = vec![0u8; 2 * PAGE_SIZE];
    let buf_write = vec![1u8; 2 * PAGE_SIZE];
    let mut reader = dma_coherent.reader();
    reader.read(&mut buf_read.as_mut_slice().into());
    assert_eq!(buf_read, buf_write);
}

#[ktest]
fn test_dma_stream_map() {
    let segment = FrameAllocOptions::new()
        .alloc_segment_with(1, |_| ())
        .unwrap();
    let dma_stream =
        DmaStream::map(segment.clone().into(), DmaDirection::Bidirectional, true).unwrap();
    assert_eq!(dma_stream.paddr(), segment.start_paddr());
    assert_eq!(dma_stream.nbytes(), PAGE_SIZE);
    assert_eq!(dma_stream.direction(), DmaDirection::Bidirectional);
}

#[ktest]
fn test_dma_stream_duplicate_map() {
    let segment_parent = FrameAllocOptions::new()
        .alloc_segment_with(2, |_| ())
        .unwrap();
    let segment_child = segment_parent.slice(&(0..PAGE_SIZE));
    let dma_stream_parent =
        DmaStream::map(segment_parent.into(), DmaDirection::Bidirectional, false).unwrap();
    let dma_stream_child = DmaStream::map(segment_child.into(), DmaDirection::Bidirectional, false);
    assert!(dma_stream_child.is_err());
}

#[ktest]
fn test_dma_stream_read_write() {
    let segment = FrameAllocOptions::new()
        .alloc_segment_with(2, |_| ())
        .unwrap();
    let dma_stream = DmaStream::map(segment.into(), DmaDirection::Bidirectional, false).unwrap();

    let buf_write = vec![1u8; 2 * PAGE_SIZE];
    dma_stream.write_bytes(0, &buf_write).unwrap();
    dma_stream.sync(0..2 * PAGE_SIZE).unwrap();
    let mut buf_read = vec![0u8; 2 * PAGE_SIZE];
    dma_stream.read_bytes(0, &mut buf_read).unwrap();
    assert_eq!(buf_write, buf_read);
}

#[ktest]
fn test_dma_stream_reader_writer() {
    let segment = FrameAllocOptions::new()
        .alloc_segment_with(2, |_| ())
        .unwrap();
    let dma_stream = DmaStream::map(segment.into(), DmaDirection::Bidirectional, false).unwrap();

    let buf_write = vec![1u8; PAGE_SIZE];
    let mut writer = dma_stream.writer().unwrap();
    writer.write(&mut buf_write.as_slice().into());
    writer.write(&mut buf_write.as_slice().into());
    dma_stream.sync(0..2 * PAGE_SIZE).unwrap();
    let mut buf_read = vec![0u8; 2 * PAGE_SIZE];
    let buf_write = vec![1u8; 2 * PAGE_SIZE];
    let mut reader = dma_stream.reader().unwrap();
    reader.read(&mut buf_read.as_mut_slice().into());
    assert_eq!(buf_read, buf_write);
}

#[ktest]
fn test_dma_stream_slice() {
    let segment = FrameAllocOptions::new()
        .alloc_segment_with(2, |_| ())
        .unwrap();
    let dma_stream = DmaStream::map(segment.into(), DmaDirection::Bidirectional, false).unwrap();
    let dma_stream_slice = DmaStreamSlice::new(&dma_stream, PAGE_SIZE, PAGE_SIZE);

    assert_eq!(dma_stream_slice.offset(), PAGE_SIZE);
    assert_eq!(dma_stream_slice.nbytes(), PAGE_SIZE);
    assert_eq!(dma_stream_slice.paddr(), dma_stream.paddr() + PAGE_SIZE);
    assert_eq!(dma_stream_slice.daddr(), dma_stream.daddr() + PAGE_SIZE);

    let buf_write = vec![1u8; PAGE_SIZE];
    dma_stream_slice.write_bytes(0, &buf_write).unwrap();
    dma_stream_slice.sync().unwrap();
    let mut buf_read = vec![0u8; PAGE_SIZE];
    dma_stream_slice.read_bytes(0, &mut buf_read).unwrap();
    assert_eq!(buf_write, buf_read);
}

#[ktest]
fn test_dma_stream_slice_reader_writer() {
    let segment = FrameAllocOptions::new()
        .alloc_segment_with(2, |_| ())
        .unwrap();
    let dma_stream = DmaStream::map(segment.into(), DmaDirection::Bidirectional, false).unwrap();
    let dma_stream_slice = DmaStreamSlice::new(&dma_stream, PAGE_SIZE, PAGE_SIZE);

    let buf_write = vec![1u8; PAGE_SIZE];
    let mut writer = dma_stream_slice.writer().unwrap();
    writer.write(&mut buf_write.as_slice().into());
    dma_stream_slice.sync().unwrap();
    let mut buf_read = vec![0u8; PAGE_SIZE];
    let mut reader = dma_stream_slice.reader().unwrap();
    reader.read(&mut buf_read.as_mut_slice().into());
    assert_eq!(buf_read, buf_write);
}

#[ktest]
#[should_panic]
fn test_dma_stream_slice_invalid_offset() {
    let segment = FrameAllocOptions::new()
        .alloc_segment_with(2, |_| ())
        .unwrap();
    let dma_stream = DmaStream::map(segment.into(), DmaDirection::Bidirectional, false).unwrap();
    let dma_stream_slice = DmaStreamSlice::new(&dma_stream, 3 * PAGE_SIZE, PAGE_SIZE);
}

#[ktest]
#[should_panic]
fn test_dma_stream_slice_invalid_len() {
    let segment = FrameAllocOptions::new()
        .alloc_segment_with(2, |_| ())
        .unwrap();
    let dma_stream = DmaStream::map(segment.into(), DmaDirection::Bidirectional, false).unwrap();
    let dma_stream_slice = DmaStreamSlice::new(&dma_stream, PAGE_SIZE, 2 * PAGE_SIZE);
}
