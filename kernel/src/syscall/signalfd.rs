// SPDX-License-Identifier: MPL-2.0

use core::sync::atomic::{AtomicBool, Ordering};

use ostd::{sync::WaitQueue, task::Task};

use super::SyscallReturn;
use crate::{
    events::{IoEvents, Observer},
    fs::{
        file_handle::FileLike,
        file_table::{get_file_fast, FdFlags, FileDesc},
        utils::{CreationFlags, InodeMode, InodeType, Metadata, StatusFlags},
    },
    prelude::*,
    process::{
        posix_thread::{AsPosixThread, PosixThread},
        signal::{
            constants::{SIGKILL, SIGSTOP}, sig_mask::{AtomicSigMask, SigMask, SigSet}, signals::Signal, PollHandle, Pollable, Pollee, SigEvents, SigEventsFilter
        },
        Gid, Uid,
    },
    time::clocks::RealTimeClock,
};

pub fn sys_signalfd(
    fd: FileDesc,
    mask_ptr: Vaddr,
    sizemask: usize,
    ctx: &Context,
) -> Result<SyscallReturn> {
    sys_signalfd4(fd, mask_ptr, sizemask, 0, ctx)
}

pub fn sys_signalfd4(
    fd: FileDesc,
    mask_ptr: Vaddr,
    sizemask: usize,
    flags: i32,
    ctx: &Context,
) -> Result<SyscallReturn> {
    debug!(
        "fd = {}, mask = {:x}, sizemask = {}, flags = {}",
        fd, mask_ptr, sizemask, flags
    );

    if sizemask != core::mem::size_of::<SigSet>() {
        return Err(Error::with_message(Errno::EINVAL, "invalid mask size"));
    }

    let mut read_mask = ctx.user_space().read_val::<SigMask>(mask_ptr)?;
    read_mask -= SIGKILL;
    read_mask -= SIGSTOP;
    let mask = !read_mask;

    let flags = SignalFileFlags::from_bits(flags as u32)
        .ok_or_else(|| Error::with_message(Errno::EINVAL, "invalid flags"))?;

    let fd_flags = if flags.contains(SignalFileFlags::O_CLOEXEC) {
        FdFlags::CLOEXEC
    } else {
        FdFlags::empty()
    };

    let non_blocking = flags.contains(SignalFileFlags::O_NONBLOCK);

    let new_fd = if fd == -1 {
        let sig_mask = AtomicSigMask::from(mask);
        let signal_file = Arc::new(SignalFile::new(
            Arc::downgrade(&ctx.thread.task()),
            sig_mask,
            non_blocking,
        ));

        // Register observer to current thread's signal queues
        let observer = Arc::downgrade(&signal_file) as Weak<dyn Observer<SigEvents>>;
        let filter = SigEventsFilter::new(mask);
        ctx.posix_thread
            .register_sigqueue_observer(observer.clone(), filter);
        *signal_file.observer().lock() = Some(observer);

        let file_table = ctx.thread_local.file_table().borrow_mut();
        let fd = file_table.write().insert(signal_file, fd_flags);
        fd
    } else {
        let mut file_table = ctx.thread_local.file_table().borrow_mut();
        let file = get_file_fast!(&mut file_table, fd);
        let signal_file = file
            .downcast_ref::<SignalFile>()
            .ok_or(Error::with_message(Errno::EINVAL, "not a signalfd"))?;

        let new_mask = AtomicSigMask::from(mask);
        if signal_file.mask().load(Ordering::Relaxed) != new_mask.load(Ordering::Relaxed) {
            // Update mask and re-register observer to associated thread
            let old_observer = signal_file.observer().lock().take();
            if let Some(task) = signal_file.task() {
                let thread = task.as_posix_thread().unwrap();
                if let Some(old_observer) = old_observer {
                    thread.unregister_sigqueue_observer(&old_observer);
                    let filter = SigEventsFilter::new(new_mask.load(Ordering::Relaxed));
                    thread.register_sigqueue_observer(old_observer.clone(), filter);
                    *signal_file.observer().lock() = Some(old_observer);
                }
            }
            signal_file
                .mask()
                .store(new_mask.load(Ordering::Relaxed), Ordering::Relaxed);
        }
        signal_file.set_non_blocking(non_blocking);
        fd
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
    task: Weak<Task>,
    mask: AtomicSigMask,
    pollee: Pollee,
    non_blocking: AtomicBool,
    observer: Mutex<Option<Weak<dyn Observer<SigEvents>>>>,
    wait_queue: WaitQueue,
}

impl SignalFile {
    fn new(task: Weak<Task>, mask: AtomicSigMask, non_blocking: bool) -> Self {
        Self {
            task,
            mask,
            pollee: Pollee::new(),
            non_blocking: AtomicBool::new(non_blocking),
            observer: Mutex::new(None),
            wait_queue: WaitQueue::new(),
        }
    }

    fn mask(&self) -> &AtomicSigMask {
        &self.mask
    }

    fn observer(&self) -> &Mutex<Option<Weak<dyn Observer<SigEvents>>>> {
        &self.observer
    }

    fn task(&self) -> Option<Arc<Task>> {
        self.task.upgrade()
    }

    fn set_non_blocking(&self, non_blocking: bool) {
        self.non_blocking.store(non_blocking, Ordering::Relaxed);
    }

    fn is_non_blocking(&self) -> bool {
        self.non_blocking.load(Ordering::Relaxed)
    }

    fn check_io_events(&self) -> IoEvents {
        if let Some(task) = self.task() {
            let thread = match task.as_posix_thread() {
                Some(t) => t,
                None => return IoEvents::empty(),
            };
            let mask = self.mask.load(Ordering::Relaxed);
            let pending = thread.sig_pending();

            if pending.intersects(mask) {
                IoEvents::IN
            } else {
                IoEvents::empty()
            }
        } else {
            IoEvents::empty()
        }
    }

    fn try_read(&self, writer: &mut VmWriter) -> Result<usize> {
        if let Some(task) = self.task() {
            let thread = match task.as_posix_thread() {
                Some(t) => t,
                None => return_errno!(Errno::ESRCH),
            };
            let mask = self.mask.load(Ordering::Relaxed);
            let buffer_size = writer.avail();
            if buffer_size % core::mem::size_of::<SignalfdSiginfo>() != 0 {
                return_errno!(Errno::EINVAL);
            }

            let max_signals = buffer_size / core::mem::size_of::<SignalfdSiginfo>();
            let mut count = 0;

            while count < max_signals {
                let signal = thread.dequeue_signal(&mask);
                debug!("try_read: signal = {:?}, mask = {:x}", signal, mask);
                if let Some(signal) = signal {
                    let info = signal.to_signalfd_siginfo();
                    writer.write_val(&info)?;
                    count += 1;
                } else {
                    break;
                }
            }

            if count == 0 {
                Err(Error::with_message(Errno::EAGAIN, "no pending signals"))
            } else {
                Ok(count * core::mem::size_of::<SignalfdSiginfo>())
            }
        } else {
            return_errno!(Errno::ESRCH);
        }
    }
}

impl Observer<SigEvents> for SignalFile {
    fn on_events(&self, events: &SigEvents) {
        debug!("[ddd] signal file received events: {:?}", events);
        let mask = self.mask.load(Ordering::Relaxed);
        if mask.contains(events.sig_num()) {
            self.pollee.notify(IoEvents::IN);
            self.wait_queue.wake_all();
        }
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
        if writer.avail() < core::mem::size_of::<SignalfdSiginfo>() {
            return_errno_with_message!(Errno::EINVAL, "buffer too small");
        }

        if self.is_non_blocking() {
            self.try_read(writer)
        } else {
            self.wait_queue.wait_until(|| Some(self.try_read(writer)))
        }
    }

    fn write(&self, _reader: &mut VmReader) -> Result<usize> {
        return_errno_with_message!(Errno::EBADF, "signalfd is not writable");
    }

    fn status_flags(&self) -> StatusFlags {
        if self.is_non_blocking() {
            StatusFlags::O_NONBLOCK
        } else {
            StatusFlags::empty()
        }
    }

    fn set_status_flags(&self, new_flags: StatusFlags) -> Result<()> {
        self.set_non_blocking(new_flags.contains(StatusFlags::O_NONBLOCK));
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
            mode: InodeMode::from_bits_truncate(0o400),
            nlinks: 1,
            uid: Uid::new_root(),
            gid: Gid::new_root(),
            rdev: 0,
        }
    }
}

impl Drop for SignalFile {
    fn drop(&mut self) {
        if let Some(observer) = self.observer().lock().take() {
            if let Some(thread) = self.task() {
                thread
                    .as_posix_thread()
                    .map(|t| t.unregister_sigqueue_observer(&observer));
            }
        }
    }
}

#[repr(C)]
#[derive(Debug, Copy, Clone, Pod)]
struct SignalfdSiginfo {
    ssi_signo: u32,
    ssi_errno: i32,
    ssi_code: i32,
    ssi_pid: u32,
    ssi_uid: u32,
    ssi_fd: i32,
    ssi_tid: u32,
    ssi_band: u32,
    ssi_overrun: u32,
    ssi_trapno: u32,
    ssi_status: i32,
    ssi_int: i32,
    ssi_ptr: u64,
    ssi_utime: u64,
    ssi_stime: u64,
    ssi_addr: u64,
    _pad: [u8; 48],
}

trait ToSignalfdSiginfo {
    fn to_signalfd_siginfo(&self) -> SignalfdSiginfo;
}

impl ToSignalfdSiginfo for Box<dyn Signal> {
    fn to_signalfd_siginfo(&self) -> SignalfdSiginfo {
        let siginfo = self.to_info();
        SignalfdSiginfo {
            ssi_signo: siginfo.si_signo as _,
            ssi_errno: siginfo.si_errno,
            ssi_code: siginfo.si_code,
            ssi_pid: 0,
            ssi_uid: 0,
            ssi_fd: 0,
            ssi_tid: 0,
            ssi_band: 0,
            ssi_overrun: 0,
            ssi_trapno: 0,
            ssi_status: 0,
            ssi_int: 0,
            ssi_ptr: 0,
            ssi_utime: 0,
            ssi_stime: 0,
            ssi_addr: 0,
            _pad: [0; 48],
        }
    }
}
