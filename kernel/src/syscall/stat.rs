// SPDX-License-Identifier: MPL-2.0

use core::time::Duration;

use super::SyscallReturn;
use crate::{
    fs::{
        file_table::FileDesc,
        fs_resolver::{FsPath, AT_FDCWD},
        utils::Metadata,
    },
    prelude::*,
    syscall::constants::MAX_FILENAME_LEN,
    time::timespec_t,
};

pub fn sys_fstat(fd: FileDesc, stat_buf_ptr: Vaddr, ctx: &Context) -> Result<SyscallReturn> {
    debug!("fd = {}, stat_buf_addr = 0x{:x}", fd, stat_buf_ptr);

    let file = {
        let file_table = ctx.process.file_table().lock();
        file_table.get_file(fd)?.clone()
    };

    let stat = Stat::from(file.metadata());
    ctx.user_space().write_val(stat_buf_ptr, &stat)?;
    Ok(SyscallReturn::Return(0))
}

pub fn sys_stat(filename_ptr: Vaddr, stat_buf_ptr: Vaddr, ctx: &Context) -> Result<SyscallReturn> {
    self::sys_fstatat(AT_FDCWD, filename_ptr, stat_buf_ptr, 0, ctx)
}

pub fn sys_lstat(filename_ptr: Vaddr, stat_buf_ptr: Vaddr, ctx: &Context) -> Result<SyscallReturn> {
    self::sys_fstatat(
        AT_FDCWD,
        filename_ptr,
        stat_buf_ptr,
        StatFlags::AT_SYMLINK_NOFOLLOW.bits(),
        ctx,
    )
}

pub fn sys_fstatat(
    dirfd: FileDesc,
    filename_ptr: Vaddr,
    stat_buf_ptr: Vaddr,
    flags: u32,
    ctx: &Context,
) -> Result<SyscallReturn> {
    let user_space = ctx.user_space();
    let filename = user_space.read_cstring(filename_ptr, MAX_FILENAME_LEN)?;
    let flags =
        StatFlags::from_bits(flags).ok_or(Error::with_message(Errno::EINVAL, "invalid flags"))?;
    debug!(
        "dirfd = {}, filename = {:?}, stat_buf_ptr = 0x{:x}, flags = {:?}",
        dirfd, filename, stat_buf_ptr, flags
    );

    if filename.is_empty() {
        if !flags.contains(StatFlags::AT_EMPTY_PATH) {
            return_errno_with_message!(Errno::ENOENT, "path is empty");
        }
        // In this case, the behavior of fstatat() is similar to that of fstat().
        return self::sys_fstat(dirfd, stat_buf_ptr, ctx);
    }

    let dentry = {
        let filename = filename.to_string_lossy();
        let fs_path = FsPath::new(dirfd, filename.as_ref())?;
        let fs = ctx.process.fs().read();
        if flags.contains(StatFlags::AT_SYMLINK_NOFOLLOW) {
            fs.lookup_no_follow(&fs_path)?
        } else {
            fs.lookup(&fs_path)?
        }
    };
    let stat = Stat::from(dentry.metadata());
    user_space.write_val(stat_buf_ptr, &stat)?;
    Ok(SyscallReturn::Return(0))
}

pub fn sys_statx(
    dirfd: FileDesc,
    filename_ptr: Vaddr,
    statx_buf_ptr: Vaddr,
    flags: u32,
    mask: u32,
    ctx: &Context,
) -> Result<SyscallReturn> {
    debug!(
        "dirfd = {}, filename_ptr = 0x{:x}, statx_buf_ptr = 0x{:x}, flags = {}, mask = {}",
        dirfd, filename_ptr, statx_buf_ptr, flags, mask
    );
    if flags == 0 {
        return sys_stat(filename_ptr, statx_buf_ptr, ctx);
    }
    let user_space = ctx.user_space();
    let filename = user_space.read_cstring(filename_ptr, MAX_FILENAME_LEN)?;
    let flags = StatFlags::from_bits(flags).ok_or(Error::new(Errno::EINVAL))?;
    let mask = StatxMask::from_bits(mask).ok_or(Error::new(Errno::EINVAL))?;
    // let mask =
    //     StatxMask::from_bits(mask).unwrap_or(StatxMask::all());
    // let flags =
    //     StatFlags::from_bits(flags).unwrap_or(StatFlags::all());

    let dentry = {
        let filename = filename.to_string_lossy();
        let fs_path = FsPath::new(dirfd, filename.as_ref())?;
        let fs = ctx.process.fs().read();

        if flags.contains(StatFlags::AT_SYMLINK_NOFOLLOW) {
            fs.lookup_no_follow(&fs_path)?
        } else {
            fs.lookup(&fs_path)?
        }
    };

    let metadata = dentry.metadata();
    let statx = Statx::build(&metadata, mask);
    user_space.write_val(statx_buf_ptr, &statx)?;

    Ok(SyscallReturn::Return(0))
}

/// File Stat
#[derive(Debug, Clone, Copy, Pod, Default)]
#[repr(C)]
pub struct Stat {
    /// ID of device containing file
    st_dev: u64,
    /// Inode number
    st_ino: u64,
    /// Number of hard links
    st_nlink: usize,
    /// File type and mode
    st_mode: u32,
    /// User ID of owner
    st_uid: u32,
    /// Group ID of owner
    st_gid: u32,
    /// Padding bytes
    __pad0: u32,
    /// Device ID (if special file)
    st_rdev: u64,
    /// Total size, in bytes
    st_size: isize,
    /// Block size for filesystem I/O
    st_blksize: isize,
    /// Number of 512-byte blocks allocated
    st_blocks: isize,
    /// Time of last access
    st_atime: timespec_t,
    /// Time of last modification
    st_mtime: timespec_t,
    /// Time of last status change
    st_ctime: timespec_t,
    /// Unused field
    __unused: [i64; 3],
}

impl From<Metadata> for Stat {
    fn from(info: Metadata) -> Self {
        Self {
            st_dev: info.dev,
            st_ino: info.ino,
            st_nlink: info.nlinks,
            st_mode: info.type_ as u32 | info.mode.bits() as u32,
            st_uid: info.uid.into(),
            st_gid: info.gid.into(),
            __pad0: 0,
            st_rdev: info.rdev,
            st_size: info.size as isize,
            st_blksize: info.blk_size as isize,
            st_blocks: (info.blocks * (info.blk_size / 512)) as isize, // Number of 512B blocks
            st_atime: info.atime.into(),
            st_mtime: info.mtime.into(),
            st_ctime: info.ctime.into(),
            __unused: [0; 3],
        }
    }
}

bitflags::bitflags! {
    struct StatFlags: u32 {
        /// Do not follow symbolic links
        const AT_SYMLINK_NOFOLLOW = 1 << 8;
        /// Do not automount
        const AT_NO_AUTOMOUNT = 1 << 11;
        /// Allow empty path
        const AT_EMPTY_PATH = 1 << 12;
        /// Retrieve mount ID
        const AT_STATX_DONT_SYNC = 1 << 13;
        /// Perform a lazy sync
        const AT_STATX_FORCE_SYNC = 1 << 14;
    }
}

#[repr(C)]
#[derive(Debug, Default, Clone, Copy, Pod)]
pub struct Statx {
    pub stx_mask: u32,
    pub stx_blksize: u32,
    pub stx_attributes: u64,
    pub stx_nlink: u32,
    pub stx_uid: u32,
    pub stx_gid: u32,
    pub stx_mode: u16,
    pub __spare0: [u16; 1],
    pub stx_ino: u64,
    pub stx_size: u64,
    pub stx_blocks: u64,
    pub stx_attributes_mask: u64,
    pub stx_atime: StatxTimestamp,
    pub stx_mtime: StatxTimestamp,
    pub stx_ctime: StatxTimestamp,
    pub stx_btime: StatxTimestamp,
    pub stx_dev_major: u32,
    pub stx_dev_minor: u32,
    pub stx_rdev_major: u32,
    pub stx_rdev_minor: u32,
    pub __spare2: [u64; 14],
}

#[repr(C)]
#[derive(Debug, Default, Clone, Copy, Pod)]
pub struct StatxTimestamp {
    pub tv_sec: i64,  // Seconds
    pub tv_nsec: u32, // Nanoseconds
    pub __reserved: u32,
}

impl From<Duration> for StatxTimestamp {
    fn from(duration: Duration) -> StatxTimestamp {
        let sec = duration.as_secs() as i64;
        let nsec = duration.subsec_nanos();
        StatxTimestamp {
            tv_sec: sec,
            tv_nsec: nsec,
            __reserved: 0,
        }
    }
}

bitflags! {
    pub struct StatxMask: u32 {
        const STATX_TYPE       = 0x00000001;
        const STATX_MODE       = 0x00000002;
        const STATX_NLINK      = 0x00000004;
        const STATX_UID        = 0x00000008;
        const STATX_GID        = 0x00000010;
        const STATX_ATIME      = 0x00000020;
        const STATX_MTIME      = 0x00000040;
        const STATX_CTIME      = 0x00000080;
        const STATX_INO        = 0x00000100;
        const STATX_SIZE       = 0x00000200;
        const STATX_BLOCKS     = 0x00000400;
        const STATX_BASIC_STATS= 0x000007ff;
        const STATX_BTIME      = 0x00000800;
        const STATX_MNT_ID     = 0x00001000;
        const STATX_DIOALIGN   = 0x00002000;
        const STATX_MNT_ID_UNIQUE = 0x00004000;
        const STATX_SUBVOL     = 0x00008000;
        const STATX_WRITE_ATOMIC = 0x00010000;
        const STATX_RESERVED   = 0x80000000;
        const STATX_ALL        = 0x00000fff;
    }
}

impl Statx {
    pub fn build(info: &Metadata, mask: StatxMask) -> Self {
        let mut statx = Statx::default();
        statx.stx_mask = mask.bits();

        if mask.contains(StatxMask::STATX_TYPE) {
            statx.stx_ino = info.ino;
            statx.stx_mode = info.type_ as u16 | info.mode.bits() as u16;
        }
        if mask.contains(StatxMask::STATX_NLINK) {
            statx.stx_nlink = info.nlinks as u32;
        }
        if mask.contains(StatxMask::STATX_UID) {
            statx.stx_uid = info.uid.into();
        }
        if mask.contains(StatxMask::STATX_GID) {
            statx.stx_gid = info.gid.into();
        }
        if mask.contains(StatxMask::STATX_INO) {
            statx.stx_ino = info.ino;
        }
        if mask.contains(StatxMask::STATX_SIZE) {
            statx.stx_size = info.size as u64;
        }
        if mask.contains(StatxMask::STATX_BLOCKS) {
            statx.stx_blocks = (info.blocks * (info.blk_size / 512)) as u64;
        }
        if mask.contains(StatxMask::STATX_ATIME) {
            statx.stx_atime = StatxTimestamp::default();
        }
        if mask.contains(StatxMask::STATX_MTIME) {
            statx.stx_mtime = StatxTimestamp::default();
        }
        if mask.contains(StatxMask::STATX_CTIME) {
            statx.stx_ctime = StatxTimestamp::default();
        }
        if mask.contains(StatxMask::STATX_BTIME) {
            // FIXME: Not supported yet
            statx.stx_btime = StatxTimestamp::default();
        }

        statx
    }
}
