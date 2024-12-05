// SPDX-License-Identifier: MPL-2.0

use ascii::AsAsciiStr;
use ostd::mm::Infallible;

use super::SyscallReturn;
use crate::{fs::file_table::FileDesc, prelude::*, syscall::read};

pub fn sys_write(
    fd: FileDesc,
    user_buf_ptr: Vaddr,
    user_buf_len: usize,
    ctx: &Context,
) -> Result<SyscallReturn> {
    debug!(
        "fd = {}, user_buf_ptr = 0x{:x}, user_buf_len = 0x{:x}",
        fd, user_buf_ptr, user_buf_len
    );

    let file = {
        let file_table = ctx.process.file_table().lock();
        file_table.get_file(fd)?.clone()
    };
    // According to <https://man7.org/linux/man-pages/man2/write.2.html>, if
    // the user specified an empty buffer, we should detect errors by checking
    // the file descriptor. If no errors detected, return 0 successfully.
    let write_len = if user_buf_len != 0 {
        let mut reader = ctx
            .process
            .root_vmar()
            .vm_space()
            .reader(user_buf_ptr, user_buf_len)?;
        let mut data = vec![0u8; user_buf_len];
        VmWriter::<'_, Infallible>::from(data.as_mut_slice()).write_fallible(&mut reader)?;
        debug!("[fff] data: {:?}", data.as_ascii_str());
        let mut tmp: VmReader<'_, Infallible> = VmReader::<'_, Infallible>::from(data.as_slice());
        file.write(&mut tmp.to_fallible())?
    } else {
        debug!("[fff] write empty buffer");
        file.write_bytes(&[])?
    };
    debug!("[fff] finish write, fd = {}, write_len = {}", fd, write_len);
    Ok(SyscallReturn::Return(write_len as _))
}
