use core::sync::atomic::{AtomicBool, Ordering};
use ostd::sync::WaitQueue;
use super::SyscallReturn;
use crate::{
    events::IoEvents,
    fs::{
        file_handle::FileLike,
        file_table::{FdFlags, FileDesc},
        utils::{CreationFlags, InodeMode, InodeType, Metadata, StatusFlags},
    },
    prelude::*,
    process::{
        signal::{c_types::sigaction_t, sig_mask::{AtomicSigMask, SigSet}, sig_num, PollHandle, Pollable, Pollee},
        Gid, Uid,
    },
    time::clocks::RealTimeClock,
};

pub fn sys_signalfd(fd: FileDesc, mask: u64, sizemask: usize, ctx: &Context) -> Result<SyscallReturn> {
    debug!("fd = {}, mask = {:?}, sizemask = {}", fd, mask, sizemask);
    let mask = AtomicSigMask::from(SigSet::from(mask));
    let signal_file = if fd == -1 {
        Arc::new(SignalFile::new(mask))
    } else {
        let mut file_table = ctx.posix_thread.file_table().lock();
        let file = file_table.get_file(fd)?.clone();
        if let Some(signal_file) = file.downcast_ref::<SignalFile>() {
            signal_file.update_mask(mask);
            file.clone()
        } else {
            return_errno_with_message!(Errno::EINVAL, "file descriptor is not a signalfd");
        }
    };

    let new_fd = {
        let mut file_table = ctx.posix_thread.file_table().lock();
        file_table.insert(signal_file, FdFlags::empty())
    };

    Ok(SyscallReturn::Return(new_fd as _))
}

pub fn sys_signalfd4(fd: FileDesc, mask: u64, sizemask: usize, flags: i32, ctx: &Context) -> Result<SyscallReturn> {
    debug!("fd = {}, mask = {:?}, sizemask = {}, flags = {}", fd, mask, sizemask, flags);

    let mask = AtomicSigMask::from(SigSet::from(mask));
    // Parse flags
    let flags = SignalFileFlags::from_bits(flags as u32)
        .ok_or_else(|| Error::with_message(Errno::EINVAL, "unknown flags"))?;

    // Handle O_CLOEXEC and O_NONBLOCK flags
    let fd_flags = if flags.contains(SignalFileFlags::O_CLOEXEC) {
        FdFlags::CLOEXEC
    } else {
        FdFlags::empty()
    };

    let non_blocking = flags.contains(SignalFileFlags::O_NONBLOCK);

    // Create or update the SignalFile
    let signal_file = if fd == -1 {
        let mut signal_file = SignalFile::new(mask);
        signal_file.set_nonblocking(non_blocking);
        Arc::new(signal_file)
    } else {
        let mut file_table = ctx.posix_thread.file_table().lock();
        let file = file_table.get_file(fd)?.clone();
        if let Some(signal_file) = file.downcast_ref::<SignalFile>() {
            signal_file.update_mask(mask);
            signal_file.set_nonblocking(non_blocking);
            file.clone()
        } else {
            return_errno_with_message!(Errno::EINVAL, "file descriptor is not a signalfd");
        }
    };

    // Insert the SignalFile into the file table
    let new_fd = {
        let mut file_table = ctx.posix_thread.file_table().lock();
        file_table.insert(signal_file, fd_flags)
    };

    Ok(SyscallReturn::Return(new_fd as _))
}

bitflags! {
    struct SignalFileFlags: u32 {
        const O_CLOEXEC = CreationFlags::O_CLOEXEC.bits();
        const O_NONBLOCK = StatusFlags::O_NONBLOCK.bits();
    }
}

struct SignalFile {
    mask: Mutex<AtomicSigMask>,
    pollee: Pollee,
    wait_queue: WaitQueue,
    non_blocking: AtomicBool,
}

impl SignalFile {
    fn new(mask: AtomicSigMask) -> Self {
        let mask = Mutex::new(mask);
        let pollee = Pollee::new();
        let wait_queue = WaitQueue::new();
        Self {
            mask,
            pollee,
            wait_queue,
            non_blocking: AtomicBool::new(false),
        }
    }

    fn update_mask(&self, new_mask: AtomicSigMask) {
        let mut mask = self.mask.lock();
        *mask = new_mask;
    }

    fn set_nonblocking(&self, non_blocking: bool) {
        self.non_blocking.store(non_blocking, Ordering::SeqCst);
    }

    fn is_nonblocking(&self) -> bool {
        self.non_blocking.load(Ordering::SeqCst)
    }

    fn check_io_events(&self) -> IoEvents {
        let mask = self.mask.lock();
        let mut events = IoEvents::empty();

        // Check if there are any pending signals that match the mask
        if !mask.load(Ordering::SeqCst).contains(SigSet::new_empty()) {
            events |= IoEvents::IN;
        }

        events
    }

    fn try_read(&self, writer: &mut VmWriter) -> Result<()> {
        let mask = self.mask.lock();

        // Wait until there are pending signals that match the mask
        if !mask.load(Ordering::SeqCst).contains(SigSet::new_empty()) {
            return_errno_with_message!(Errno::EAGAIN, "no pending signals");
        }

        // Read the next pending signal
        let signal_info = self.get_next_signal_info()?;
        writer.write_fallible(&mut signal_info)?;

        self.pollee.notify(IoEvents::IN);

        Ok(())
    }

    fn get_next_signal_info(&self) -> Result<SignalfdSiginfo> {
        // This is a placeholder for the actual logic to retrieve the next signal info.
        Ok(SignalfdSiginfo {
            ssi_signo: 2,      // Example: SIGINT
            ssi_errno: 0,      // Unused
            ssi_code: 0,       // Unused
            ssi_pid: 12345,    // Example PID
            ssi_uid: 1000,     // Example UID
            ..Default::default()
        })
    }
}

impl Pollable for SignalFile {
    fn poll(&self, mask: IoEvents, poller: Option<&mut PollHandle>) -> IoEvents {
        self.pollee
            .poll_with(mask, poller, || self.check_io_events())
    }
}

impl FileLike for SignalFile {
    fn read(&self, writer: &mut VmWriter) -> Result<usize> {
        let read_len = core::mem::size_of::<SignalfdSiginfo>();

        if writer.avail() < read_len {
            return_errno_with_message!(Errno::EINVAL, "buf len is less than the size of signalfd_siginfo");
        }

        if self.is_nonblocking() {
            self.try_read(writer)?;
        } else {
            self.wait_events(IoEvents::IN, None, || self.try_read(writer))?;
        }

        Ok(read_len)
    }

    fn write(&self, _reader: &mut VmReader) -> Result<usize> {
        return_errno_with_message!(Errno::EINVAL, "signalfd is not writable");
    }

    fn status_flags(&self) -> StatusFlags {
        if self.is_nonblocking() {
            StatusFlags::O_NONBLOCK
        } else {
            StatusFlags::empty()
        }
    }

    fn set_status_flags(&self, new_flags: StatusFlags) -> Result<()> {
        if new_flags.contains(StatusFlags::O_NONBLOCK) {
            self.set_nonblocking(true);
        } else {
            self.set_nonblocking(false);
        }
        Ok(())
    }

    fn metadata(&self) -> Metadata {
        let now = RealTimeClock::get().read_time();
        Metadata {
            dev: 0,
            ino: 0,
            size: 0,
            blk_size: 0,
            blocks: 0,
            atime: now,
            mtime: now,
            ctime: now,
            type_: InodeType::NamedPipe,
            mode: InodeMode::from_bits_truncate(0o200),
            nlinks: 1,
            uid: Uid::new_root(),
            gid: Gid::new_root(),
            rdev: 0,
        }
    }
}

#[repr(C)]
#[derive(Debug, Default, Copy, Clone)]
struct SignalfdSiginfo {
    ssi_signo: u32,    // Signal number
    ssi_errno: i32,    // Error number (unused)
    ssi_code: i32,     // Signal code
    ssi_pid: u32,      // PID of sender
    ssi_uid: u32,      // UID of sender
    ssi_fd: i32,       // File descriptor (for SIGIO)
    ssi_tid: u32,      // Kernel timer ID
    ssi_band: u32,     // Band event (for SIGPOLL)
    ssi_overrun: u32,  // POSIX timer overrun count
    ssi_trapno: u32,   // Trap number that caused signal
    ssi_status: i32,   // Exit status or signal
    ssi_int: i32,      // Integer sent by sigqueue(2)
    ssi_ptr: u64,      // Pointer sent by sigqueue(2)
    ssi_utime: u64,    // User CPU time consumed
    ssi_stime: u64,    // System CPU time consumed
    ssi_addr: u64,     // Address that generated signal
    _pad: [u8; 48],    // Padding to 128 bytes
}


