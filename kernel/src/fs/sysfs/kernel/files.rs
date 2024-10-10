// SPDX-License-Identifier: MPL-2.0
#![allow(unused)]

use ostd::mm::VmWriter;

use crate::{fs::kernfs::DataProvider, prelude::*};

pub struct AddressBits;

impl DataProvider for AddressBits {
    fn read_at(&self, writer: &mut VmWriter, offset: usize) -> Result<usize> {
        let data = "64\n".as_bytes().to_vec();
        let start = data.len().min(offset);
        let end = data.len().min(offset + writer.avail());
        let len = end - start;
        writer.write_fallible(&mut (&data[start..end]).into())?;
        Ok(len)
    }

    fn write_at(&mut self, _reader: &mut VmReader, _offset: usize) -> Result<usize> {
        return_errno_with_message!(Errno::EINVAL, "cpuinfo is read-only");
    }
}

pub struct CpuByteOrder {
    byte_order: Vec<u8>,
}

impl CpuByteOrder {
    pub fn new() -> Self {
        let byte_order = if cfg!(target_endian = "big") {
            "big\n".as_bytes().to_vec()
        } else {
            "little\n".as_bytes().to_vec()
        };
        Self { byte_order }
    }
}

impl Default for CpuByteOrder {
    fn default() -> Self {
        Self::new()
    }
}

impl DataProvider for CpuByteOrder {
    fn read_at(&self, writer: &mut VmWriter, offset: usize) -> Result<usize> {
        let start = self.byte_order.len().min(offset);
        let end = self.byte_order.len().min(offset + writer.avail());
        let len = end - start;
        writer.write_fallible(&mut (&self.byte_order[start..end]).into())?;
        Ok(len)
    }

    fn write_at(&mut self, reader: &mut VmReader, offset: usize) -> Result<usize> {
        let write_len = reader.remain();
        let end = offset + write_len;

        if self.byte_order.len() < end {
            self.byte_order.resize(end, 0);
        }

        let mut writer = VmWriter::from(&mut self.byte_order[offset..end]);
        let value = reader.read_fallible(&mut writer)?;
        if value != write_len {
            return_errno!(Errno::EINVAL);
        }

        Ok(write_len)
    }
}
