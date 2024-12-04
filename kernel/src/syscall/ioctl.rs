// SPDX-License-Identifier: MPL-2.0

use super::SyscallReturn;
use crate::{
    fs::{
        file_table::{FdFlags, FileDesc},
        utils::{IoctlCmd, StatusFlags},
    },
    prelude::*,
};

pub fn sys_ioctl(fd: FileDesc, cmd: u32, arg: Vaddr, ctx: &Context) -> Result<SyscallReturn> {
    let ioctl_cmd = IoctlCmd::try_from(cmd)?;
    debug!(
        "fd = {}, ioctl_cmd = {:?}, arg = 0x{:x}",
        fd, ioctl_cmd, arg
    );

    let file = {
        let file_table = ctx.posix_thread.file_table().lock();
        file_table.get_file(fd)?.clone()
    };
    let res = match ioctl_cmd {
        IoctlCmd::FIONBIO => {
            let is_nonblocking = ctx.user_space().read_val::<i32>(arg)? != 0;
            let mut flags = file.status_flags();
            flags.set(StatusFlags::O_NONBLOCK, is_nonblocking);
            file.set_status_flags(flags)?;
            0
        }
        IoctlCmd::FIOASYNC => {
            let is_async = ctx.user_space().read_val::<i32>(arg)? != 0;
            let mut flags = file.status_flags();

            // Set `O_ASYNC` flags will send `SIGIO` signal to a process when
            // I/O is possible, user should call `fcntl(fd, F_SETOWN, pid)`
            // first to let the kernel know just whom to notify.
            flags.set(StatusFlags::O_ASYNC, is_async);
            file.set_status_flags(flags)?;
            0
        }
        IoctlCmd::FIOCLEX => {
            // Sets the close-on-exec flag of the file.
            // Follow the implementation of fcntl()

            let flags = FdFlags::CLOEXEC;
            let file_table = ctx.posix_thread.file_table().lock();
            let entry = file_table.get_entry(fd)?;
            entry.set_flags(flags);
            0
        }
        IoctlCmd::FIONCLEX => {
            // Clears the close-on-exec flag of the file.
            let file_table = ctx.posix_thread.file_table().lock();
            let entry = file_table.get_entry(fd)?;
            entry.set_flags(entry.flags() & (!FdFlags::CLOEXEC));
            0
        }
        IoctlCmd::SIOCGIFCONF => {
            // Read the IfConf structure from user space
            let ifconf  = ctx.user_space().read_val::<IfConf>(arg)?;
            debug!("SIOCGIFCONF: ifconf = {:?}", ifconf);
            let buffer_ptr = ifconf.ifc_buf;
            let buffer_len = ifconf.ifc_len as usize;

            // Define the interface name and IP for localhost
            let if_name = b"lo\0\0\0\0\0\0\0\0\0\0\0\0\0\0"; // 16 bytes
            let ip_addr = 0x0100007Fu32; // 127.0.0.1 in little endian

            // Create the IfReq structure
            let mut ifreq = IfReq {
                ifr_name: {
                    let mut name = [0u8; IFNAMSIZ];
                    name.copy_from_slice(&if_name[..IFNAMSIZ]);
                    name
                },
                ifr_union: {
                    // Set the IP address in ifr_union (assuming the first 4 bytes)
                    let mut union = [0u8; 24];
                    union[..4].copy_from_slice(&ip_addr.to_le_bytes());
                    union
                },
            };

            // Ensure the buffer is large enough to hold one IfReq
            let ifreq_size = core::mem::size_of::<IfReq>();
            if buffer_len < ifreq_size {
                return_errno!(Errno::EINVAL);
            }

            // Write the IfReq structure to the user buffer
            ctx.user_space().write_val(buffer_ptr, &ifreq)?;
            debug!("SIOCGIFCONF: ifreq = {:?}", ifreq);

            // Update the ifc_len to reflect the number of bytes written
            let updated_ifconf = IfConf {
                ifc_len: ifreq_size as i32,
                ifc_buf: ifconf.ifc_buf, // Buffer address remains the same
            };
            ctx.user_space().write_val(arg, &updated_ifconf)?;
            debug!("SIOCGIFCONF: updated ifconf = {:?}", updated_ifconf);
            // Return the number of bytes written
            ifreq_size as _
        }
        _ => file.ioctl(ioctl_cmd, arg)?,
    };
    Ok(SyscallReturn::Return(res as _))
}

#[derive(Debug, Pod, Copy, Clone)]
#[repr(C)]
pub struct IfConf {
    pub ifc_len: i32,
    pub ifc_buf: Vaddr,
}

const IFNAMSIZ: usize = 16;
#[derive(Debug, Pod, Copy, Clone)]
#[repr(C)]
pub struct IfReq {
    pub ifr_name: [u8; IFNAMSIZ],
    pub ifr_union: [u8; 24],
}