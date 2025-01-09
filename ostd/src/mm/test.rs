// SPDX-License-Identifier: MPL-2.0

use alloc::vec;
use core::mem::size_of;

use crate::{
    mm::io::{VmReader, VmWriter},
    prelude::*,
    Error,
};

mod io {
    use ostd_pod::Pod;

    use super::*;
    use crate::mm::{FallibleVmRead, FallibleVmWrite, FrameAllocOptions, VmIo, VmSpace};

    // A dummy Pod struct for testing complex types.
    #[repr(C)]
    #[derive(Clone, Copy, PartialEq, Debug, Pod)]
    pub struct TestPodStruct {
        pub a: u32,
        pub b: u64,
    }

    /// Test reading and writing u32 values using VmReader and VmWriter in Infallible mode.
    #[ktest]
    fn read_write_u32_infallible() {
        let mut buffer = vec![0u8; 8];
        let writer = VmWriter::from(&mut buffer[..]);

        let mut writer_infallible =
            unsafe { VmWriter::from_kernel_space(writer.cursor(), buffer.len()) };

        // Write two u32 values
        let val1: u32 = 0xDEADBEEF;
        let val2: u32 = 0xFEEDC0DE;

        writer_infallible.write_val(&val1).unwrap();
        writer_infallible.write_val(&val2).unwrap();

        assert_eq!(&buffer[..4], &val1.to_le_bytes()[..]);
        assert_eq!(&buffer[4..], &val2.to_le_bytes()[..]);

        // Read back the values
        let reader = VmReader::from(&buffer[..]);
        let mut reader_infallible =
            unsafe { VmReader::from_kernel_space(reader.cursor(), buffer.len()) };

        let read_val1: u32 = reader_infallible.read_val().unwrap();
        let read_val2: u32 = reader_infallible.read_val().unwrap();

        assert_eq!(val1, read_val1);
        assert_eq!(val2, read_val2);
    }

    /// Test reading and writing slices using VmReader and VmWriter in Infallible mode.
    #[ktest]
    fn read_write_slice_infallible() {
        let data = [1u8, 2, 3, 4, 5];
        let mut buffer = vec![0u8; data.len()];
        let writer = VmWriter::from(&mut buffer[..]);

        let mut writer_infallible =
            unsafe { VmWriter::from_kernel_space(writer.cursor(), buffer.len()) };

        writer_infallible.write(&mut VmReader::from(&data[..]));

        assert_eq!(buffer, data);

        // Read back the bytes
        let reader = VmReader::from(&buffer[..]);
        let mut reader_infallible =
            unsafe { VmReader::from_kernel_space(reader.cursor(), buffer.len()) };

        let mut read_buffer = [0u8; 5];
        reader_infallible.read(&mut VmWriter::from(&mut read_buffer[..]));

        assert_eq!(read_buffer, data);
    }

    /// Test writing and reading a struct using VmWriter and VmReader in Infallible mode.
    #[ktest]
    fn read_write_struct_infallible() {
        let mut buffer = vec![0u8; size_of::<TestPodStruct>()];
        let writer = VmWriter::from(&mut buffer[..]);

        let mut writer_infallible =
            unsafe { VmWriter::from_kernel_space(writer.cursor(), buffer.len()) };

        let test_struct = TestPodStruct {
            a: 0x12345678,
            b: 0xABCDEF0123456789,
        };
        writer_infallible.write_val(&test_struct).unwrap();

        // Read back the struct
        let reader = VmReader::from(&buffer[..]);
        let mut reader_infallible =
            unsafe { VmReader::from_kernel_space(reader.cursor(), buffer.len()) };

        let read_struct: TestPodStruct = reader_infallible.read_val().unwrap();

        assert_eq!(test_struct, read_struct);
    }

    /// Test attempting to read beyond the buffer in Infallible mode.
    #[ktest]
    #[should_panic]
    fn read_beyond_buffer_infallible() {
        let buffer = [1u8, 2, 3];
        let reader = VmReader::from(&buffer[..]);
        let mut reader_infallible =
            unsafe { VmReader::from_kernel_space(reader.cursor(), buffer.len()) };

        // Attempt to read a u32 which requires 4 bytes, but buffer has only 3
        let _val: u32 = reader_infallible.read_val().unwrap();
    }

    /// Test writing beyond the buffer in Infallible mode.
    #[ktest]
    #[should_panic]
    fn write_beyond_buffer_infallible() {
        let mut buffer = vec![0u8; 3];
        let writer = VmWriter::from(&mut buffer[..]);
        let mut writer_infallible =
            unsafe { VmWriter::from_kernel_space(writer.cursor(), buffer.len()) };

        // Attempt to write a u32 which requires 4 bytes, but buffer has only 3
        let val: u32 = 0xDEADBEEF;
        writer_infallible.write_val(&val).unwrap();
    }

    /// Test the `fill` method in VmWriter in Infallible mode.
    #[ktest]
    fn fill_infallible() {
        let mut buffer = vec![0u8; 8];
        let writer = VmWriter::from(&mut buffer[..]);
        let mut writer_infallible =
            unsafe { VmWriter::from_kernel_space(writer.cursor(), buffer.len()) };

        // Fill with 0xFF
        let filled = writer_infallible.fill(0xFFu8);
        assert_eq!(filled, 8);
        assert_eq!(buffer, vec![0xFF; 8]);

        // Ensure the cursor is at the end
        assert_eq!(writer_infallible.avail(), 0);
    }

    /// Test the `skip` method in VmReader in Infallible mode.
    #[ktest]
    fn skip_read_infallible() {
        let data = [10u8, 20, 30, 40, 50];
        let reader = VmReader::from(&data[..]);
        let mut reader_infallible =
            unsafe { VmReader::from_kernel_space(reader.cursor(), reader.remain()) };

        // Skip first two bytes
        reader_infallible = reader_infallible.skip(2);

        // Read the remaining bytes
        let mut read_buffer = [0u8; 3];
        reader_infallible.read(&mut VmWriter::from(&mut read_buffer[..]));

        assert_eq!(read_buffer, [30, 40, 50]);
    }

    /// Test the `skip` method in VmWriter in Infallible mode.
    #[ktest]
    fn skip_write_infallible() {
        let mut buffer = vec![0u8; 5];
        let writer = VmWriter::from(&mut buffer[..]);
        let mut writer_infallible =
            unsafe { VmWriter::from_kernel_space(writer.cursor(), writer.avail()) };

        // Skip first two bytes
        writer_infallible = writer_infallible.skip(2);

        // Write [100, 101, 102]
        let data = [100u8, 101, 102];
        writer_infallible.write(&mut VmReader::from(&data[..]));

        assert_eq!(buffer, [0, 0, 100, 101, 102]);
    }

    /// Test the `limit` method in VmReader.
    #[ktest]
    fn limit_read_infallible() {
        let data = [1u8, 2, 3, 4, 5];
        let reader = VmReader::from(&data[..]);
        let mut limited_reader = reader.limit(3);

        assert_eq!(limited_reader.remain(), 3);

        let mut read_buffer = [0u8; 3];
        limited_reader.read(&mut VmWriter::from(&mut read_buffer[..]));
        assert_eq!(read_buffer, [1, 2, 3]);

        // Ensure no more data can be read
        let mut extra_buffer = [0u8; 1];
        let extra_read = limited_reader.read(&mut VmWriter::from(&mut extra_buffer[..]));
        assert_eq!(extra_read, 0);
    }

    /// Test the `limit` method in VmWriter.
    #[ktest]
    fn limit_write_infallible() {
        let mut buffer = vec![0u8; 5];
        let writer = VmWriter::from(&mut buffer[..]);
        let mut limited_writer = writer.limit(3);

        assert_eq!(limited_writer.avail(), 3);

        // Write [10, 20, 30, 40] but only first three should be written
        let data = [10u8, 20, 30, 40];
        for val in data.iter() {
            let _ = limited_writer.write_val(val);
        }
        assert_eq!(buffer, [10, 20, 30, 0, 0]);
    }

    /// Test the `read_slice` and `write_slice` methods in VmIo.
    #[ktest]
    fn read_write_slice_vmio_infallible() {
        let data = [100u8, 101, 102, 103, 104];
        let mut buffer = vec![0u8; 5];
        let writer = VmWriter::from(&mut buffer[..]);

        let mut writer_infallible =
            unsafe { VmWriter::from_kernel_space(writer.cursor(), buffer.len()) };
        writer_infallible.write(&mut VmReader::from(&data[..]));

        assert_eq!(buffer, data);

        let reader = VmReader::from(&buffer[..]);
        let mut reader_infallible =
            unsafe { VmReader::from_kernel_space(reader.cursor(), buffer.len()) };

        let mut read_data = [0u8; 5];
        reader_infallible.read(&mut VmWriter::from(&mut read_data[..]));

        assert_eq!(read_data, data);
    }

    /// Test the `read_once` and `write_once` methods in VmReader and VmWriter.
    #[ktest]
    fn read_write_once_infallible() {
        let mut buffer = vec![0u8; 8];
        let writer = VmWriter::from(&mut buffer[..]);
        let mut writer_infallible =
            unsafe { VmWriter::from_kernel_space(writer.cursor(), buffer.len()) };

        let val: u64 = 0x1122334455667788;
        writer_infallible.write_once(&val).unwrap();

        // Read back the value
        let reader = VmReader::from(&buffer[..]);
        let mut reader_infallible =
            unsafe { VmReader::from_kernel_space(reader.cursor(), buffer.len()) };

        let read_val: u64 = reader_infallible.read_once().unwrap();
        assert_eq!(val, read_val);
    }

    /// Test the `write_vals` method in VmWrite.
    #[ktest]
    fn write_val_infallible() {
        let mut buffer = vec![0u8; 12];
        let writer = VmWriter::from(&mut buffer[..]);
        let mut writer_infallible =
            unsafe { VmWriter::from_kernel_space(writer.cursor(), buffer.len()) };

        let values = [1u32, 2, 3];
        for val in values.iter() {
            writer_infallible.write_val(val).unwrap();
        }
        assert_eq!(buffer, [1, 0, 0, 0, 2, 0, 0, 0, 3, 0, 0, 0]);
    }

    /// Test the `FallbackVmRead` and `FallbackVmWrite` traits (using Fallible mode).
    /// Note: Since simulating page faults is non-trivial in a test environment,
    /// we'll focus on successful read and write operations.
    #[ktest]
    fn fallible_read_write() {
        let mut buffer = vec![0u8; 8];
        let writer = VmWriter::from(&mut buffer[..]);
        let mut writer_fallible = writer.to_fallible();

        let val: u64 = 0xAABBCCDDEEFF0011;
        assert!(writer_fallible.has_avail());
        writer_fallible.write_val(&val).unwrap();

        // Read back the value
        let reader = VmReader::from(&buffer[..]);
        let mut reader_fallible = reader.to_fallible();

        assert!(reader_fallible.has_remain());
        let read_val: u64 = reader_fallible.read_val().unwrap();
        assert_eq!(val, read_val);
    }

    /// Test partial read in FallibleVmRead.
    /// Since we cannot simulate page faults, we'll mimic partial reads by limiting the reader.
    #[ktest]
    fn partial_read_fallible() {
        let data = [10u8, 20, 30, 40, 50];
        let reader = VmReader::from(&data[..]);
        let reader_fallible = reader.to_fallible();

        // Limit the reader to 3 bytes
        let mut limited_reader = reader_fallible.limit(3);

        let mut writer_buffer = vec![0u8; 5];
        let writer = VmWriter::from(&mut writer_buffer[..]);
        let mut writer_fallible = writer.to_fallible();

        // Attempt to read 5 bytes into a writer limited to 3 bytes
        let result = limited_reader.read_fallible(&mut writer_fallible);

        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 3);
        assert_eq!(&writer_buffer[..3], &[10, 20, 30]);
    }

    /// Test partial write in FallibleVmWrite.
    /// Since we cannot simulate page faults, we'll mimic partial writes by limiting the writer.
    /// Note: This test is similar to `test_partial_read_fallible`, but with writer instead of reader.
    #[ktest]
    fn partial_write_fallible() {
        let mut buffer = vec![0u8; 5];
        let writer = VmWriter::from(&mut buffer[..]);
        let writer_fallible = writer.to_fallible();

        // Limit the writer to 3 bytes
        let mut limited_writer = writer_fallible.limit(3);

        let data = [10u8, 20, 30, 40, 50];
        let mut reader = VmReader::from(&data[..]);

        // Attempt to write 5 bytes into a writer limited to 3 bytes
        let result = limited_writer.write_fallible(&mut reader);

        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 3);
        assert_eq!(&buffer[..3], &[10, 20, 30]);
    }

    // Test `write_val` method and `read_val` method in Fallible mode.
    #[ktest]
    fn read_write_val_fallible() {
        let mut buffer = vec![0u8; 8];
        let writer = VmWriter::from(&mut buffer[..]);
        let mut writer_fallible = writer.to_fallible();

        let val: u64 = 0xAABBCCDDEEFF0011;
        writer_fallible.write_val(&val).unwrap();

        // Read back the value
        let reader = VmReader::from(&buffer[..]);
        let mut reader_fallible = reader.to_fallible();

        let read_val: u64 = reader_fallible.read_val().unwrap();
        assert_eq!(val, read_val);
    }

    /// Test `collect` method in VmReader.
    #[ktest]
    fn collect_fallible() {
        let data = [5u8, 6, 7, 8, 9];
        let reader = VmReader::from(&data[..]);
        let mut reader_fallible = reader.to_fallible();

        let collected = reader_fallible.collect().unwrap();
        assert_eq!(collected, data);
    }

    /// Test `collect` method with partial read in FallibleVmRead.
    #[ktest]
    fn collect_partial_fallible() {
        let data = [1u8, 2, 3, 4, 5];
        let reader = VmReader::from(&data[..]);
        let reader_fallible = reader.to_fallible();

        // Limit the reader to 3 bytes
        let mut limited_reader = reader_fallible.limit(3);

        let result = limited_reader.collect();
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), vec![1, 2, 3]);
    }

    /// Test `fill_zeros` method in VmWriter.
    #[ktest]
    fn fill_zeros_fallible() {
        let mut buffer = vec![1u8; 8];
        let writer = VmWriter::from(&mut buffer[..]);
        let mut writer_fallible = writer.to_fallible();

        writer_fallible.fill_zeros(8).unwrap();
        assert_eq!(buffer, vec![0u8; 8]);
    }

    /// Test invalid args on read/write in FallibleVmRead.
    #[ktest]
    fn invalid_args_read_write_fallible() {
        let mut buffer = vec![0u8; 3];
        let writer = VmWriter::from(&mut buffer[..]);
        let mut writer_fallible = writer.to_fallible();

        // Attempt to write a u32 which requires 4 bytes, but buffer has only 3
        let val: u32 = 0xDEADBEEF;
        let result = writer_fallible.write_val(&val);
        assert_eq!(result, Err(Error::InvalidArgs));

        let reader = VmReader::from(&buffer[..]);
        let mut reader_fallible = reader.to_fallible();

        // Attempt to read a u32 which requires 4 bytes, but buffer has only 3
        let result = reader_fallible.read_val::<u32>();
        assert_eq!(result, Err(Error::InvalidArgs));
    }

    /// Test invalid reader/writer on read/write in FallibleVmRead.
    #[ktest]
    fn invalid_reader_writer_fallible() {
        let vmspace = Arc::new(VmSpace::new());
        vmspace.activate();
        let mut reader_fallible = vmspace.reader(0x4000, 10).unwrap();
        let mut writer_fallible = vmspace.writer(0x4000, 10).unwrap();

        let result = writer_fallible.write_val(&0xDEADBEEFu32);
        assert_eq!(result, Err(Error::PageFault));
        let result = reader_fallible.read_val::<u32>();
        assert_eq!(result, Err(Error::PageFault));
    }

    /// Test invalid collect in Fallible mode.
    #[ktest]
    fn invalid_collect_fallible() {
        let vmspace = Arc::new(VmSpace::new());
        vmspace.activate();
        let mut reader_fallible = vmspace.reader(0x4000, 10).unwrap();
        let result = reader_fallible.collect();
        assert_eq!(result, Err(Error::PageFault));
    }

    /// Test invalid read and write in Infallible mode.
    #[ktest]
    fn invalid_read_write_infallible() {
        let mut buffer = vec![0u8; 3];
        let writer = VmWriter::from(&mut buffer[..]);
        let mut writer_infallible =
            unsafe { VmWriter::from_kernel_space(writer.cursor(), buffer.len()) };

        // Attempt to write a u32 which requires 4 bytes, but buffer has only 3
        let val: u32 = 0xDEADBEEF;
        let result = writer_infallible.write_val(&val);
        assert_eq!(result, Err(Error::InvalidArgs));

        let reader = VmReader::from(&buffer[..]);
        let mut reader_infallible =
            unsafe { VmReader::from_kernel_space(reader.cursor(), buffer.len()) };

        // Attempt to read a u32 which requires 4 bytes, but buffer has only 3
        let result = reader_infallible.read_val::<u32>();
        assert_eq!(result, Err(Error::InvalidArgs));
    }

    /// Test `write_vals` method in VmIO.
    #[ktest]
    fn write_vals_segment() {
        let mut buffer = vec![0u8; 12];
        let segment = FrameAllocOptions::new().alloc_segment(1).unwrap();
        let values = [1u32, 2, 3];
        let nr_written = segment.write_vals(0, values.iter(), 4).unwrap();
        assert_eq!(nr_written, 3);
        segment.read_bytes(0, &mut buffer[..]).unwrap();
        assert_eq!(buffer, [1, 0, 0, 0, 2, 0, 0, 0, 3, 0, 0, 0]);
        // Write with error offset
        let result = segment.write_vals(8192, values.iter(), 4);
        assert_eq!(result, Err(Error::InvalidArgs));
    }

    /// Test `write_slice` method in VmIO.
    #[ktest]
    fn write_slice_segment() {
        let mut buffer = vec![0u8; 12];
        let segment = FrameAllocOptions::new().alloc_segment(1).unwrap();
        let data = [1u8, 2, 3, 4, 5];
        segment.write_slice(0, &data[..]).unwrap();
        segment.read_bytes(0, &mut buffer[..]).unwrap();
        assert_eq!(buffer[..5], data);
    }

    /// Test `read_val` method in VmIO.
    #[ktest]
    fn read_val_segment() {
        let segment = FrameAllocOptions::new().alloc_segment(1).unwrap();
        let values = [1u32, 2, 3];
        segment.write_vals(0, values.iter(), 4).unwrap();
        let val: u32 = segment.read_val(0).unwrap();
        assert_eq!(val, 1);
    }

    /// Test `read_slice` method in VmIO.
    #[ktest]
    fn read_slice_segment() {
        let segment = FrameAllocOptions::new().alloc_segment(1).unwrap();
        let values = [1u32, 2, 3];
        segment.write_vals(0, values.iter(), 4).unwrap();
        let mut read_buffer = [0u8; 12];
        segment.read_slice(0, &mut read_buffer[..]).unwrap();
        assert_eq!(read_buffer, [1, 0, 0, 0, 2, 0, 0, 0, 3, 0, 0, 0]);
    }
}

mod page_prop {
    use alloc::format;

    use super::*;
    use crate::mm::{CachePolicy, PageFlags, PageProperty, PrivilegedPageFlags};

    /// Test whether the `PageProperty::new` method correctly creates a `PageProperty` instance.
    #[ktest]
    fn page_property_new() {
        let flags = PageFlags::R | PageFlags::W;
        let cache = CachePolicy::Writeback;
        let page_property = PageProperty::new(flags, cache);

        assert_eq!(page_property.flags, flags);
        assert_eq!(page_property.cache, cache);
        assert_eq!(page_property.priv_flags, PrivilegedPageFlags::USER);
    }

    /// Test whether the `PageProperty::new_absent` method correctly creates an invalid `PageProperty`.
    #[ktest]
    fn page_property_new_absent() {
        let page_property = PageProperty::new_absent();

        assert_eq!(page_property.flags, PageFlags::empty());
        assert_eq!(page_property.cache, CachePolicy::Writeback);
        assert_eq!(page_property.priv_flags, PrivilegedPageFlags::empty());
    }

    /// Test each variant of the `CachePolicy` enum.
    #[ktest]
    fn cache_policy_enum() {
        assert_eq!(CachePolicy::Uncacheable as u8, 0);
        assert_eq!(CachePolicy::WriteCombining as u8, 1);
        assert_eq!(CachePolicy::WriteProtected as u8, 2);
        assert_eq!(CachePolicy::Writethrough as u8, 3);
        assert_eq!(CachePolicy::Writeback as u8, 4);
    }

    /// Test the basic functionality of `PageFlags` bitflags.
    #[ktest]
    fn page_flags_basic() {
        let flags = PageFlags::R;
        assert!(flags.contains(PageFlags::R));
        assert!(!flags.contains(PageFlags::W));
        assert!(!flags.contains(PageFlags::X));

        let flags = PageFlags::RWX;
        assert!(flags.contains(PageFlags::R));
        assert!(flags.contains(PageFlags::W));
        assert!(flags.contains(PageFlags::X));
    }

    /// Test whether combinations of `PageFlags` are correct.
    #[ktest]
    fn page_flags_combinations() {
        let rw = PageFlags::R | PageFlags::W;
        assert_eq!(rw, PageFlags::RW);

        let rx = PageFlags::R | PageFlags::X;
        assert_eq!(rx, PageFlags::RX);

        let rwx = PageFlags::R | PageFlags::W | PageFlags::X;
        assert_eq!(rwx, PageFlags::RWX);
    }

    /// Test the accessed and dirty bits of `PageFlags`.
    #[ktest]
    fn page_flags_accessed_dirty() {
        let flags = PageFlags::ACCESSED;
        assert!(flags.contains(PageFlags::ACCESSED));
        assert!(!flags.contains(PageFlags::DIRTY));

        let flags = PageFlags::DIRTY;
        assert!(flags.contains(PageFlags::DIRTY));
        assert!(!flags.contains(PageFlags::ACCESSED));

        let flags = PageFlags::ACCESSED | PageFlags::DIRTY;
        assert!(flags.contains(PageFlags::ACCESSED));
        assert!(flags.contains(PageFlags::DIRTY));
    }

    /// Test the available bits of `PageFlags`.
    #[ktest]
    fn page_flags_available() {
        let flags = PageFlags::AVAIL1;
        assert!(flags.contains(PageFlags::AVAIL1));
        assert!(!flags.contains(PageFlags::AVAIL2));

        let flags = PageFlags::AVAIL2;
        assert!(flags.contains(PageFlags::AVAIL2));
        assert!(!flags.contains(PageFlags::AVAIL1));

        let flags = PageFlags::AVAIL1 | PageFlags::AVAIL2;
        assert!(flags.contains(PageFlags::AVAIL1));
        assert!(flags.contains(PageFlags::AVAIL2));
    }

    /// Test the basic functionality of `PrivilegedPageFlags`.
    #[ktest]
    fn privileged_page_flags_basic() {
        let flags = PrivilegedPageFlags::USER;
        assert!(flags.contains(PrivilegedPageFlags::USER));
        assert!(!flags.contains(PrivilegedPageFlags::GLOBAL));

        let flags = PrivilegedPageFlags::GLOBAL;
        assert!(flags.contains(PrivilegedPageFlags::GLOBAL));
        assert!(!flags.contains(PrivilegedPageFlags::USER));
    }

    /// Test combinations of `PrivilegedPageFlags`.
    #[ktest]
    fn privileged_page_flags_combinations() {
        let flags = PrivilegedPageFlags::USER | PrivilegedPageFlags::GLOBAL;
        // Since `bitflags` implements `Debug` and `PartialEq` for `PrivilegedPageFlags`, we can directly compare
        let expected = PrivilegedPageFlags::USER | PrivilegedPageFlags::GLOBAL;
        assert_eq!(flags, expected);
    }

    /// Test the `PrivilegedPageFlags::SHARED` flag (only under specific configurations).
    #[ktest]
    #[cfg(all(target_arch = "x86_64", feature = "cvm_guest"))]
    fn privileged_page_flags_shared_enabled() {
        let flags = PrivilegedPageFlags::SHARED;
        assert!(flags.contains(PrivilegedPageFlags::SHARED));
    }

    /// Test that the `PrivilegedPageFlags::SHARED` flag is unavailable when conditions are not met.
    #[ktest]
    #[cfg(not(all(target_arch = "x86_64", feature = "cvm_guest")))]
    fn privileged_page_flags_shared_disabled() {
        // Since the `SHARED` flag is undefined when conditions are not met,
        // we cannot directly test its absence, but we can ensure the code compiles.
        let flags = PrivilegedPageFlags::USER | PrivilegedPageFlags::GLOBAL;
        assert!(flags.contains(PrivilegedPageFlags::USER));
        assert!(flags.contains(PrivilegedPageFlags::GLOBAL));
    }

    /// Test the Debug output of `PageProperty`.
    #[ktest]
    fn page_property_debug() {
        let flags = PageFlags::RW | PageFlags::DIRTY;
        let cache = CachePolicy::WriteProtected;
        let page_property = PageProperty::new(flags, cache);

        let debug_str = format!("{:?}", page_property);
        assert!(debug_str.contains("flags"));
        assert!(debug_str.contains("RW"));
        assert!(debug_str.contains("DIRTY"));
        assert!(debug_str.contains("WriteProtected"));
    }

    /// Test the Clone and Copy traits for `PageFlags`.
    #[ktest]
    fn page_flags_clone_copy() {
        let flags = PageFlags::R | PageFlags::X;
        let cloned_flags = flags.clone();
        let copied_flags = flags;

        assert_eq!(flags, cloned_flags);
        assert_eq!(flags, copied_flags);
    }

    /// Test the Clone and Copy traits for `PageProperty`.
    #[ktest]
    fn page_property_clone_copy() {
        let flags = PageFlags::RX;
        let cache = CachePolicy::Writethrough;
        let page_property1 = PageProperty::new(flags, cache);
        let page_property2 = page_property1.clone();
        let page_property3 = page_property1;

        assert_eq!(page_property1, page_property2);
        assert_eq!(page_property1, page_property3);
    }

    /// Test the PartialEq and Eq implementations for `PageProperty`.
    #[ktest]
    fn page_property_equality() {
        let flags1 = PageFlags::R | PageFlags::W;
        let cache1 = CachePolicy::Writeback;
        let page_property1 = PageProperty::new(flags1, cache1);

        let flags2 = PageFlags::R | PageFlags::W;
        let cache2 = CachePolicy::Writeback;
        let page_property2 = PageProperty::new(flags2, cache2);

        assert_eq!(page_property1, page_property2);

        let page_property3 = PageProperty::new_absent();
        assert_ne!(page_property1, page_property3);
    }

    /// Test bit operations for `PageFlags`.
    #[ktest]
    fn page_flags_bit_operations() {
        let mut flags = PageFlags::empty();
        flags.insert(PageFlags::R);
        assert!(flags.contains(PageFlags::R));
        assert!(!flags.contains(PageFlags::W));

        flags.insert(PageFlags::W);
        assert!(flags.contains(PageFlags::R));
        assert!(flags.contains(PageFlags::W));

        flags.remove(PageFlags::R);
        assert!(!flags.contains(PageFlags::R));
        assert!(flags.contains(PageFlags::W));
    }

    /// Test bit operations for `PrivilegedPageFlags`.
    #[ktest]
    fn privileged_page_flags_bit_operations() {
        let mut flags = PrivilegedPageFlags::empty();
        flags.insert(PrivilegedPageFlags::USER);
        assert!(flags.contains(PrivilegedPageFlags::USER));
        assert!(!flags.contains(PrivilegedPageFlags::GLOBAL));

        flags.insert(PrivilegedPageFlags::GLOBAL);
        assert!(flags.contains(PrivilegedPageFlags::USER));
        assert!(flags.contains(PrivilegedPageFlags::GLOBAL));

        flags.remove(PrivilegedPageFlags::USER);
        assert!(!flags.contains(PrivilegedPageFlags::USER));
        assert!(flags.contains(PrivilegedPageFlags::GLOBAL));
    }
}
