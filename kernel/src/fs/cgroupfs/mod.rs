// SPDX-License-Identifier: MPL-2.0

/// A pseudo filesystem that is working as cgroupfs.
/// It takes effect if you mount it to `/sys/fs/cgroup`.
use core::{sync::atomic::{AtomicU64, Ordering}, time::Duration};
use inherit_methods_macro::inherit_methods;
use aster_rights::Full;
use ostd::mm::VmWriter;

use super::utils::{InodeMode, InodeType, IoctlCmd, Metadata};
use crate::{
    fs::{
        kernfs::{DataProvider, KernfsNode, KernfsNodeFlag, PseudoFileSystem},
        utils::{FileSystem, FsFlags, Inode, SuperBlock, NAME_MAX},
    },
    prelude::*, process::{Gid, Uid}, vm::vmo::Vmo,
};

/// CgroupFS filesystem.
/// Magic number.
const CGROUPFS_MAGIC: u64 = 0x9fa1;
/// Root Inode ID.
const CGROUPFS_ROOT_INO: u64 = 1;
/// Block size.
const BLOCK_SIZE: usize = 1024;

pub struct CgroupFS {
    sb: SuperBlock,
    root: Arc<dyn Inode>,
    inode_allocator: AtomicU64,
    this: Weak<Self>,
}

impl CgroupFS {
    pub fn new() -> Arc<Self> {
        Arc::new_cyclic(|weak_fs| Self {
            sb: SuperBlock::new(CGROUPFS_MAGIC, BLOCK_SIZE, NAME_MAX),
            root: CgroupfsRoot::new_inode(weak_fs.clone()),
            inode_allocator: AtomicU64::new(CGROUPFS_ROOT_INO + 1),
            this: weak_fs.clone(),
        })
    }
}

impl PseudoFileSystem for CgroupFS {
    fn alloc_id(&self) -> u64 {
        self.inode_allocator.fetch_add(1, Ordering::SeqCst)
    }

    /// Initializes the `cgroupfs` by creating the necessary directory hierarchy.
    fn init(&self) -> Result<()> {
        let root_inode = self.root_inode();
        let root = root_inode
            .downcast_ref::<KernfsNode>()
            .ok_or_else(|| Error::new(Errno::EINVAL))?;

        let subsystems = vec![
            ("cpu", "cpuacct", "cpu,cpuacct"),
            ("memory", "", ""),
            ("cpuset", "", ""),
            ("devices", "", ""),
            ("freezer", "", ""),
            ("net_cls", "net_prio", "net_cls,net_prio"),
            ("blkio", "", ""),
            ("perf_event", "", ""),
            ("hugetlb", "", ""),
            ("pids", "", ""),
            ("rdma", "", ""),
            ("systemd", "", ""),
        ];

        for (name, symlink1, symlink2) in subsystems {
            let subsystem = CgroupSubsystem::new(name, root.this())?;
            if !symlink1.is_empty() {
                KernfsNode::new_symlink(symlink1, KernfsNodeFlag::empty(), subsystem.this_weak(), subsystem.this_weak())?;
            }
            if !symlink2.is_empty() {
                KernfsNode::new_symlink(symlink2, KernfsNodeFlag::empty(), subsystem.this_weak(), subsystem.this_weak())?;
            }
        }

        Ok(())
    }

    fn fs(&self) -> Arc<dyn FileSystem> {
        self.this.upgrade().unwrap()
    }
}

impl FileSystem for CgroupFS {
    fn sync(&self) -> Result<()> {
        Ok(())
    }

    fn root_inode(&self) -> Arc<dyn Inode> {
        self.root.clone()
    }

    fn sb(&self) -> SuperBlock {
        self.sb.clone()
    }

    fn flags(&self) -> FsFlags {
        FsFlags::empty()
    }
}

/// Represents the inode at `/cgroup`.
/// Root directory of the cgroupfs.
pub struct CgroupfsRoot;

impl CgroupfsRoot {
    pub fn new_inode(fs: Weak<CgroupFS>) -> Arc<dyn Inode> {
        KernfsNode::new_root("cgroupfs", fs, CGROUPFS_ROOT_INO, BLOCK_SIZE)
    }
}

pub struct CgroupSubsystem(Arc<KernfsNode>);

impl CgroupSubsystem {
    pub fn new(name: &str, parent: Arc<KernfsNode>) -> Result<Arc<Self>> {
        let node = KernfsNode::new_dir(name, None, KernfsNodeFlag::empty(), parent.this_weak())?;
        
        Ok(Arc::new(Self(node)))
    }

    pub fn this_weak(&self) -> Weak<KernfsNode> {
        Arc::downgrade(&self.0)
    }
}

/// Inherit `Inode` trait from KernfsNode for `CgroupSubsystem`.
#[inherit_methods(from = "self.0")]
impl Inode for CgroupSubsystem {
    fn size(&self) -> usize;
    fn metadata(&self) -> Metadata;
    fn ino(&self) -> u64;
    fn mode(&self) -> Result<InodeMode>;
    fn set_mode(&self, mode: InodeMode) -> Result<()>;
    fn owner(&self) -> Result<Uid>;
    fn set_owner(&self, uid: Uid) -> Result<()>;
    fn group(&self) -> Result<Gid>;
    fn set_group(&self, gid: Gid) -> Result<()>;
    fn atime(&self) -> Duration;
    fn set_atime(&self, time: Duration);
    fn mtime(&self) -> Duration;
    fn set_mtime(&self, time: Duration);
    fn ctime(&self) -> Duration;
    fn set_ctime(&self, time: Duration);
    fn fs(&self) -> Arc<dyn FileSystem>;
    fn resize(&self, _new_size: usize) -> Result<()>;
    fn type_(&self) -> InodeType;
    fn read_at(&self, _offset: usize, _writer: &mut VmWriter) -> Result<usize>;
    fn read_direct_at(&self, _offset: usize, _writer: &mut VmWriter) -> Result<usize>;
    fn write_at(&self, _offset: usize, _reader: &mut VmReader) -> Result<usize>;
    fn write_direct_at(&self, _offset: usize, _reader: &mut VmReader) -> Result<usize>;
    fn read_link(&self) -> Result<String>;
    fn write_link(&self, _target: &str) -> Result<()>;
    fn ioctl(&self, _cmd: IoctlCmd, _arg: usize) -> Result<i32>;
    fn is_dentry_cacheable(&self) -> bool;
    fn page_cache(&self) -> Option<Vmo<Full>>;
    fn lookup(&self, _name: &str) -> Result<Arc<dyn Inode>>;
    fn link(&self, _inode: &Arc<dyn Inode>, _name: &str) -> Result<()>;
    fn unlink(&self, _name: &str) -> Result<()>;
    fn as_device(&self) -> Option<Arc<dyn super::device::Device>>;
    fn readdir_at(
        &self,
        offset: usize,
        visitor: &mut dyn super::utils::DirentVisitor,
    ) -> Result<usize>;
    fn rmdir(&self, name: &str) -> Result<()>;
    fn rename(&self, old_name: &str, target: &Arc<dyn Inode>, new_name: &str) -> Result<()>;
    fn sync_all(&self) -> Result<()>;
    fn sync_data(&self) -> Result<()>;
    fn fallocate(&self, mode: super::utils::FallocMode, offset: usize, len: usize) -> Result<()>;
    fn poll(
        &self,
        mask: crate::events::IoEvents,
        _poller: Option<&mut crate::process::signal::Poller>,
    ) -> crate::events::IoEvents;
    fn is_seekable(&self) -> bool;
    fn extension(&self) -> Option<&super::utils::Extension>;
    fn mknod(
        &self,
        name: &str,
        mode: super::utils::InodeMode,
        type_: super::utils::MknodType,
    ) -> Result<Arc<dyn Inode>>;

    fn create(&self, name: &str, type_: InodeType, mode: InodeMode) -> Result<Arc<dyn Inode>> {
        // 1. Check if the inode is a directory.
        // 2. Check if the name already exists.
        // 3. Create a new inode.
        // 4. Take the parent's init function to create the components of the new inode.
        // 5. Return the new inode.

        if self.0.type_() != InodeType::Dir {
            return_errno!(Errno::ENOTDIR);
        }
        if self.0.lookup(name).is_ok() {
            return_errno!(Errno::EEXIST);
        }
        let new_node = match type_ {
            InodeType::Dir => {
                KernfsNode::new_dir(name, Some(mode), KernfsNodeFlag::empty(), self.this_weak())
            }
            _ => return_errno!(Errno::EINVAL),
        }?;
        Ok(new_node)
        
    } 
}

struct CgroupSubsystemData {
    data: Vec<u8>,
}

impl CgroupSubsystemData {
    pub fn new(data: &str) -> Self {
        if !data.ends_with('\n') {
            let mut new_data = data.to_string();
            new_data.push('\n');
            Self {
                data: new_data.as_bytes().to_vec(),
            }
        } else {
            Self {
                data: data.as_bytes().to_vec(),
            }
        }
    }
}

impl DataProvider for CgroupSubsystemData {
    fn read_at(&self, writer: &mut VmWriter, offset: usize) -> Result<usize> {
        let start = self.data.len().min(offset);
        let end = self.data.len().min(offset + writer.avail());
        let len = end - start;
        writer.write_fallible(&mut (&self.data[start..end]).into())?;
        Ok(len)
    }

    fn write_at(&mut self, reader: &mut VmReader, offset: usize) -> Result<usize> {
        let write_len = reader.remain();
        let end = offset + write_len;

        if self.data.len() < end {
            self.data.resize(end, 0);
        }

        let mut writer = VmWriter::from(&mut self.data[offset..end]);
        let value = reader.read_fallible(&mut writer)?;
        if value != write_len {
            return_errno!(Errno::EINVAL);
        }

        Ok(write_len)
    }
}

struct CpuSubsystem;

impl CpuSubsystem {
    pub fn new(name: &str, parent: Arc<KernfsNode>) -> Result<Arc<KernfsNode>> {
        let mode = InodeMode::from_bits_truncate(0o555);
        let node = KernfsNode::new_dir(
            name,
            Some(mode),
            KernfsNodeFlag::empty(),
            parent.this_weak(),
        )?;

        let cchild = KernfsNode::new_attr(
            "cgroup.clone_children",
            Some(InodeMode::from_bits_truncate(0o755)),
            KernfsNodeFlag::empty(),
            node.this_weak(),
        )?;
        cchild.set_data(Box::new(CgroupSubsystemData::new("0")))?;

        let cpuacct_stat = KernfsNode::new_attr(
            "cpuacct.stat",
            Some(InodeMode::from_bits_truncate(0o755)),
            KernfsNodeFlag::empty(),
            node.this_weak(),
        )?;
        cpuacct_stat.set_data(Box::new(CgroupSubsystemData::new("user 0\nsystem 0")))?;

        let cpuacct_usage = KernfsNode::new_attr(
            "cpuacct.usage",
            Some(InodeMode::from_bits_truncate(0o755)),
            KernfsNodeFlag::empty(),
            node.this_weak(),
        )?;
        cpuacct_usage.set_data(Box::new(CgroupSubsystemData::new("0")))?;

        let cpuacct_usage_percpu = KernfsNode::new_attr(
            "cpuacct.usage_percpu",
            Some(InodeMode::from_bits_truncate(0o755)),
            KernfsNodeFlag::empty(),
            node.this_weak(),
        )?;
        cpuacct_usage_percpu.set_data(Box::new(CgroupSubsystemData::new("0")))?;

        let cpuacct_usage_percpu_sys = KernfsNode::new_attr(
            "cpuacct.usage_percpu_sys",
            Some(InodeMode::from_bits_truncate(0o755)),
            KernfsNodeFlag::empty(),
            node.this_weak(),
        )?;
        cpuacct_usage_percpu_sys.set_data(Box::new(CgroupSubsystemData::new("0")))?;

        let cpuacct_usage_percpu_user = KernfsNode::new_attr(
            "cpuacct.usage_percpu_user",
            Some(InodeMode::from_bits_truncate(0o755)),
            KernfsNodeFlag::empty(),
            node.this_weak(),
        )?;
        cpuacct_usage_percpu_user.set_data(Box::new(CgroupSubsystemData::new("0")))?;

        let cpuacct_usage_sys = KernfsNode::new_attr(
            "cpuacct.usage_sys",
            Some(InodeMode::from_bits_truncate(0o755)),
            KernfsNodeFlag::empty(),
            node.this_weak(),
        )?;
        cpuacct_usage_sys.set_data(Box::new(CgroupSubsystemData::new("0")))?;

        let cpuacct_usage_user = KernfsNode::new_attr(
            "cpuacct.usage_user",
            Some(InodeMode::from_bits_truncate(0o755)),
            KernfsNodeFlag::empty(),
            node.this_weak(),
        )?;
        cpuacct_usage_user.set_data(Box::new(CgroupSubsystemData::new("0")))?;

        let cpu_cfs_period_us = KernfsNode::new_attr(
            "cpu.cfs_period_us",
            Some(InodeMode::from_bits_truncate(0o755)),
            KernfsNodeFlag::empty(),
            node.this_weak(),
        )?;
        cpu_cfs_period_us.set_data(Box::new(CgroupSubsystemData::new("100000")))?;

        let cpu_cfs_quota_us = KernfsNode::new_attr(
            "cpu.cfs_quota_us",
            Some(InodeMode::from_bits_truncate(0o755)),
            KernfsNodeFlag::empty(),
            node.this_weak(),
        )?;
        cpu_cfs_quota_us.set_data(Box::new(CgroupSubsystemData::new("-1")))?;

        let cpu_cfs_burst_us = KernfsNode::new_attr(
            "cpu.cfs_burst_us",
            Some(InodeMode::from_bits_truncate(0o755)),
            KernfsNodeFlag::empty(),
            node.this_weak(),
        )?;
        cpu_cfs_burst_us.set_data(Box::new(CgroupSubsystemData::new("0")))?;

        let cpu_shares = KernfsNode::new_attr(
            "cpu.shares",
            Some(InodeMode::from_bits_truncate(0o755)),
            KernfsNodeFlag::empty(),
            node.this_weak(),
        )?;
        cpu_shares.set_data(Box::new(CgroupSubsystemData::new("1024")))?;

        let cpu_stat = KernfsNode::new_attr(
            "cpu.stat",
            Some(InodeMode::from_bits_truncate(0o755)),
            KernfsNodeFlag::empty(),
            node.this_weak(),
        )?;
        cpu_stat.set_data(Box::new(CgroupSubsystemData::new(
            "nr_periods 0\nnr_throttled 0\nthrottled_time 0",
        )))?;

        let notify_on_release = KernfsNode::new_attr(
            "notify_on_release",
            Some(InodeMode::from_bits_truncate(0o755)),
            KernfsNodeFlag::empty(),
            node.this_weak(),
        )?;
        notify_on_release.set_data(Box::new(CgroupSubsystemData::new("0")))?;

        let tasks = KernfsNode::new_attr(
            "tasks",
            Some(InodeMode::from_bits_truncate(0o755)),
            KernfsNodeFlag::empty(),
            node.this_weak(),
        )?;
        tasks.set_data(Box::new(CgroupSubsystemData::new("")))?;

        Ok(node)
    }
}

struct MemorySubsystem;

impl MemorySubsystem {
    pub fn new(name: &str, parent: Arc<KernfsNode>) -> Result<Arc<KernfsNode>> {
        let mode = InodeMode::from_bits_truncate(0o555);
        let node = KernfsNode::new_dir(
            name,
            Some(mode),
            KernfsNodeFlag::empty(),
            parent.this_weak(),
        )?;

        let memory_usage_in_bytes = KernfsNode::new_attr(
            "memory.usage_in_bytes",
            Some(InodeMode::from_bits_truncate(0o755)),
            KernfsNodeFlag::empty(),
            node.this_weak(),
        )?;
        memory_usage_in_bytes.set_data(Box::new(CgroupSubsystemData::new("0")))?;

        let memory_limit_in_bytes = KernfsNode::new_attr(
            "memory.limit_in_bytes",
            Some(InodeMode::from_bits_truncate(0o755)),
            KernfsNodeFlag::empty(),
            node.this_weak(),
        )?;
        memory_limit_in_bytes
            .set_data(Box::new(CgroupSubsystemData::new("9223372036854771712")))?;

        let memory_failcnt = KernfsNode::new_attr(
            "memory.failcnt",
            Some(InodeMode::from_bits_truncate(0o755)),
            KernfsNodeFlag::empty(),
            node.this_weak(),
        )?;
        memory_failcnt.set_data(Box::new(CgroupSubsystemData::new("0")))?;

        let memory_max_usage_in_bytes = KernfsNode::new_attr(
            "memory.max_usage_in_bytes",
            Some(InodeMode::from_bits_truncate(0o755)),
            KernfsNodeFlag::empty(),
            node.this_weak(),
        )?;
        memory_max_usage_in_bytes.set_data(Box::new(CgroupSubsystemData::new("0")))?;

        let memory_soft_limit_in_bytes = KernfsNode::new_attr(
            "memory.soft_limit_in_bytes",
            Some(InodeMode::from_bits_truncate(0o755)),
            KernfsNodeFlag::empty(),
            node.this_weak(),
        )?;
        memory_soft_limit_in_bytes
            .set_data(Box::new(CgroupSubsystemData::new("9223372036854771712")))?;

        let memory_kmem_usage_in_bytes = KernfsNode::new_attr(
            "memory.kmem.usage_in_bytes",
            Some(InodeMode::from_bits_truncate(0o755)),
            KernfsNodeFlag::empty(),
            node.this_weak(),
        )?;
        memory_kmem_usage_in_bytes.set_data(Box::new(CgroupSubsystemData::new("0")))?;

        let memory_kmem_limit_in_bytes = KernfsNode::new_attr(
            "memory.kmem.limit_in_bytes",
            Some(InodeMode::from_bits_truncate(0o755)),
            KernfsNodeFlag::empty(),
            node.this_weak(),
        )?;
        memory_kmem_limit_in_bytes
            .set_data(Box::new(CgroupSubsystemData::new("9223372036854771712")))?;

        let memory_kmem_failcnt = KernfsNode::new_attr(
            "memory.kmem.failcnt",
            Some(InodeMode::from_bits_truncate(0o755)),
            KernfsNodeFlag::empty(),
            node.this_weak(),
        )?;
        memory_kmem_failcnt.set_data(Box::new(CgroupSubsystemData::new("0")))?;

        let memory_kmem_max_usage_in_bytes = KernfsNode::new_attr(
            "memory.kmem.max_usage_in_bytes",
            Some(InodeMode::from_bits_truncate(0o755)),
            KernfsNodeFlag::empty(),
            node.this_weak(),
        )?;
        memory_kmem_max_usage_in_bytes.set_data(Box::new(CgroupSubsystemData::new("0")))?;

        let memory_kmem_tcp_usage_in_bytes = KernfsNode::new_attr(
            "memory.kmem.tcp.usage_in_bytes",
            Some(InodeMode::from_bits_truncate(0o755)),
            KernfsNodeFlag::empty(),
            node.this_weak(),
        )?;
        memory_kmem_tcp_usage_in_bytes.set_data(Box::new(CgroupSubsystemData::new("0")))?;

        let memory_kmem_tcp_limit_in_bytes = KernfsNode::new_attr(
            "memory.kmem.tcp.limit_in_bytes",
            Some(InodeMode::from_bits_truncate(0o755)),
            KernfsNodeFlag::empty(),
            node.this_weak(),
        )?;
        memory_kmem_tcp_limit_in_bytes
            .set_data(Box::new(CgroupSubsystemData::new("9223372036854771712")))?;

        let memory_kmem_tcp_failcnt = KernfsNode::new_attr(
            "memory.kmem.tcp.failcnt",
            Some(InodeMode::from_bits_truncate(0o755)),
            KernfsNodeFlag::empty(),
            node.this_weak(),
        )?;
        memory_kmem_tcp_failcnt.set_data(Box::new(CgroupSubsystemData::new("0")))?;

        let memory_kmem_tcp_max_usage_in_bytes = KernfsNode::new_attr(
            "memory.kmem.tcp.max_usage_in_bytes",
            Some(InodeMode::from_bits_truncate(0o755)),
            KernfsNodeFlag::empty(),
            node.this_weak(),
        )?;
        memory_kmem_tcp_max_usage_in_bytes.set_data(Box::new(CgroupSubsystemData::new("0")))?;

        Ok(node)
    }
}

struct CpusetSubsystem;

impl CpusetSubsystem {
    pub fn new(name: &str, parent: Arc<KernfsNode>) -> Result<Arc<KernfsNode>> {
        let mode = InodeMode::from_bits_truncate(0o555);
        let node = KernfsNode::new_dir(
            name,
            Some(mode),
            KernfsNodeFlag::empty(),
            parent.this_weak(),
        )?;

        let cpuset_cpus = KernfsNode::new_attr(
            "cpuset.cpus",
            Some(InodeMode::from_bits_truncate(0o755)),
            KernfsNodeFlag::empty(),
            node.this_weak(),
        )?;
        cpuset_cpus.set_data(Box::new(CgroupSubsystemData::new("0-3")))?;

        let cpuset_mems = KernfsNode::new_attr(
            "cpuset.mems",
            Some(InodeMode::from_bits_truncate(0o755)),
            KernfsNodeFlag::empty(),
            node.this_weak(),
        )?;
        cpuset_mems.set_data(Box::new(CgroupSubsystemData::new("0")))?;

        let cpuset_memory_migrate = KernfsNode::new_attr(
            "cpuset.memory_migrate",
            Some(InodeMode::from_bits_truncate(0o755)),
            KernfsNodeFlag::empty(),
            node.this_weak(),
        )?;
        cpuset_memory_migrate.set_data(Box::new(CgroupSubsystemData::new("0")))?;

        let cpuset_cpu_exclusive = KernfsNode::new_attr(
            "cpuset.cpu_exclusive",
            Some(InodeMode::from_bits_truncate(0o755)),
            KernfsNodeFlag::empty(),
            node.this_weak(),
        )?;
        cpuset_cpu_exclusive.set_data(Box::new(CgroupSubsystemData::new("0")))?;

        let cpuset_mem_exclusive = KernfsNode::new_attr(
            "cpuset.mem_exclusive",
            Some(InodeMode::from_bits_truncate(0o755)),
            KernfsNodeFlag::empty(),
            node.this_weak(),
        )?;
        cpuset_mem_exclusive.set_data(Box::new(CgroupSubsystemData::new("0")))?;

        let cpuset_mem_hardwall = KernfsNode::new_attr(
            "cpuset.mem_hardwall",
            Some(InodeMode::from_bits_truncate(0o755)),
            KernfsNodeFlag::empty(),
            node.this_weak(),
        )?;
        cpuset_mem_hardwall.set_data(Box::new(CgroupSubsystemData::new("0")))?;

        let cpuset_memory_pressure = KernfsNode::new_attr(
            "cpuset.memory_pressure",
            Some(InodeMode::from_bits_truncate(0o755)),
            KernfsNodeFlag::empty(),
            node.this_weak(),
        )?;
        cpuset_memory_pressure.set_data(Box::new(CgroupSubsystemData::new("0")))?;

        let cpuset_memory_spread_page = KernfsNode::new_attr(
            "cpuset.memory_spread_page",
            Some(InodeMode::from_bits_truncate(0o755)),
            KernfsNodeFlag::empty(),
            node.this_weak(),
        )?;
        cpuset_memory_spread_page.set_data(Box::new(CgroupSubsystemData::new("0")))?;

        let cpuset_memory_spread_slab = KernfsNode::new_attr(
            "cpuset.memory_spread_slab",
            Some(InodeMode::from_bits_truncate(0o755)),
            KernfsNodeFlag::empty(),
            node.this_weak(),
        )?;
        cpuset_memory_spread_slab.set_data(Box::new(CgroupSubsystemData::new("0")))?;

        let cpuset_sched_load_balance = KernfsNode::new_attr(
            "cpuset.sched_load_balance",
            Some(InodeMode::from_bits_truncate(0o755)),
            KernfsNodeFlag::empty(),
            node.this_weak(),
        )?;
        cpuset_sched_load_balance.set_data(Box::new(CgroupSubsystemData::new("1")))?;

        let cpuset_sched_relax_domain_level = KernfsNode::new_attr(
            "cpuset.sched_relax_domain_level",
            Some(InodeMode::from_bits_truncate(0o755)),
            KernfsNodeFlag::empty(),
            node.this_weak(),
        )?;
        cpuset_sched_relax_domain_level.set_data(Box::new(CgroupSubsystemData::new("-1")))?;

        Ok(node)
    }
}

struct DevicesSubsystem;

impl DevicesSubsystem {
    pub fn new(name: &str, parent: Arc<KernfsNode>) -> Result<Arc<KernfsNode>> {
        let mode = InodeMode::from_bits_truncate(0o555);
        let node = KernfsNode::new_dir(
            name,
            Some(mode),
            KernfsNodeFlag::empty(),
            parent.this_weak(),
        )?;

        let devices_allow = KernfsNode::new_attr(
            "devices.allow",
            Some(InodeMode::from_bits_truncate(0o755)),
            KernfsNodeFlag::empty(),
            node.this_weak(),
        )?;
        devices_allow.set_data(Box::new(CgroupSubsystemData::new("")))?;

        let devices_deny = KernfsNode::new_attr(
            "devices.deny",
            Some(InodeMode::from_bits_truncate(0o755)),
            KernfsNodeFlag::empty(),
            node.this_weak(),
        )?;
        devices_deny.set_data(Box::new(CgroupSubsystemData::new("")))?;

        let devices_list = KernfsNode::new_attr(
            "devices.list",
            Some(InodeMode::from_bits_truncate(0o755)),
            KernfsNodeFlag::empty(),
            node.this_weak(),
        )?;
        devices_list.set_data(Box::new(CgroupSubsystemData::new("")))?;

        let devices_log_level = KernfsNode::new_attr(
            "devices.log_level",
            Some(InodeMode::from_bits_truncate(0o755)),
            KernfsNodeFlag::empty(),
            node.this_weak(),
        )?;
        devices_log_level.set_data(Box::new(CgroupSubsystemData::new("")))?;

        let devices_max_count = KernfsNode::new_attr(
            "devices.max_count",
            Some(InodeMode::from_bits_truncate(0o755)),
            KernfsNodeFlag::empty(),
            node.this_weak(),
        )?;
        devices_max_count.set_data(Box::new(CgroupSubsystemData::new("")))?;

        let devices_priority = KernfsNode::new_attr(
            "devices.priority",
            Some(InodeMode::from_bits_truncate(0o755)),
            KernfsNodeFlag::empty(),
            node.this_weak(),
        )?;
        devices_priority.set_data(Box::new(CgroupSubsystemData::new("")))?;

        Ok(node)
    }
}

struct FreezerSubsystem;

impl FreezerSubsystem {
    pub fn new(name: &str, parent: Arc<KernfsNode>) -> Result<Arc<KernfsNode>> {
        let mode = InodeMode::from_bits_truncate(0o555);
        let node = KernfsNode::new_dir(
            name,
            Some(mode),
            KernfsNodeFlag::empty(),
            parent.this_weak(),
        )?;

        let freezer_state = KernfsNode::new_attr(
            "freezer.state",
            Some(InodeMode::from_bits_truncate(0o755)),
            KernfsNodeFlag::empty(),
            node.this_weak(),
        )?;
        freezer_state.set_data(Box::new(CgroupSubsystemData::new("THAWED")))?;

        let freezer_self_freezing = KernfsNode::new_attr(
            "freezer.self_freezing",
            Some(InodeMode::from_bits_truncate(0o755)),
            KernfsNodeFlag::empty(),
            node.this_weak(),
        )?;
        freezer_self_freezing.set_data(Box::new(CgroupSubsystemData::new("0")))?;

        let freezer_parent_freezing = KernfsNode::new_attr(
            "freezer.parent_freezing",
            Some(InodeMode::from_bits_truncate(0o755)),
            KernfsNodeFlag::empty(),
            node.this_weak(),
        )?;
        freezer_parent_freezing.set_data(Box::new(CgroupSubsystemData::new("0")))?;

        Ok(node)
    }
}

struct HugetlbSubsystem;

impl HugetlbSubsystem {
    pub fn new(name: &str, parent: Arc<KernfsNode>) -> Result<Arc<KernfsNode>> {
        let mode = InodeMode::from_bits_truncate(0o555);
        let node = KernfsNode::new_dir(
            name,
            Some(mode),
            KernfsNodeFlag::empty(),
            parent.this_weak(),
        )?;

        let hugetlb_max = KernfsNode::new_attr(
            "hugetlb.max",
            Some(InodeMode::from_bits_truncate(0o755)),
            KernfsNodeFlag::empty(),
            node.this_weak(),
        )?;
        hugetlb_max.set_data(Box::new(CgroupSubsystemData::new("0")))?;

        let hugetlb_current = KernfsNode::new_attr(
            "hugetlb.current",
            Some(InodeMode::from_bits_truncate(0o755)),
            KernfsNodeFlag::empty(),
            node.this_weak(),
        )?;
        hugetlb_current.set_data(Box::new(CgroupSubsystemData::new("0")))?;

        let hugetlb_failcnt = KernfsNode::new_attr(
            "hugetlb.failcnt",
            Some(InodeMode::from_bits_truncate(0o755)),
            KernfsNodeFlag::empty(),
            node.this_weak(),
        )?;
        hugetlb_failcnt.set_data(Box::new(CgroupSubsystemData::new("0")))?;

        let hugetlb_limit_in_bytes = KernfsNode::new_attr(
            "hugetlb.limit_in_bytes",
            Some(InodeMode::from_bits_truncate(0o755)),
            KernfsNodeFlag::empty(),
            node.this_weak(),
        )?;
        hugetlb_limit_in_bytes.set_data(Box::new(CgroupSubsystemData::new("0")))?;

        let hugetlb_usage_in_bytes = KernfsNode::new_attr(
            "hugetlb.usage_in_bytes",
            Some(InodeMode::from_bits_truncate(0o755)),
            KernfsNodeFlag::empty(),
            node.this_weak(),
        )?;
        hugetlb_usage_in_bytes.set_data(Box::new(CgroupSubsystemData::new("0")))?;

        Ok(node)
    }
}

struct NetClsSubsystem;

impl NetClsSubsystem {
    pub fn new(name: &str, parent: Arc<KernfsNode>) -> Result<Arc<KernfsNode>> {
        let mode = InodeMode::from_bits_truncate(0o555);
        let node = KernfsNode::new_dir(
            name,
            Some(mode),
            KernfsNodeFlag::empty(),
            parent.this_weak(),
        )?;

        let net_cls_classid = KernfsNode::new_attr(
            "net_cls.classid",
            Some(InodeMode::from_bits_truncate(0o755)),
            KernfsNodeFlag::empty(),
            node.this_weak(),
        )?;
        net_cls_classid.set_data(Box::new(CgroupSubsystemData::new("0")))?;

        let net_cls_cgroups = KernfsNode::new_attr(
            "net_cls.cgroups",
            Some(InodeMode::from_bits_truncate(0o755)),
            KernfsNodeFlag::empty(),
            node.this_weak(),
        )?;
        net_cls_cgroups.set_data(Box::new(CgroupSubsystemData::new("0")))?;

        let net_cls_mark = KernfsNode::new_attr(
            "net_cls.mark",
            Some(InodeMode::from_bits_truncate(0o755)),
            KernfsNodeFlag::empty(),
            node.this_weak(),
        )?;
        net_cls_mark.set_data(Box::new(CgroupSubsystemData::new("0")))?;

        Ok(node)
    }
}

struct PerfEventSubsystem;

impl PerfEventSubsystem {
    pub fn new(name: &str, parent: Arc<KernfsNode>) -> Result<Arc<KernfsNode>> {
        let mode = InodeMode::from_bits_truncate(0o555);
        let node = KernfsNode::new_dir(
            name,
            Some(mode),
            KernfsNodeFlag::empty(),
            parent.this_weak(),
        )?;

        let perf_event_enable = KernfsNode::new_attr(
            "perf_event.enable",
            Some(InodeMode::from_bits_truncate(0o755)),
            KernfsNodeFlag::empty(),
            node.this_weak(),
        )?;
        perf_event_enable.set_data(Box::new(CgroupSubsystemData::new("0")))?;

        let perf_event_events = KernfsNode::new_attr(
            "perf_event.events",
            Some(InodeMode::from_bits_truncate(0o755)),
            KernfsNodeFlag::empty(),
            node.this_weak(),
        )?;
        perf_event_events.set_data(Box::new(CgroupSubsystemData::new("0")))?;

        let perf_event_inherit = KernfsNode::new_attr(
            "perf_event.inherit",
            Some(InodeMode::from_bits_truncate(0o755)),
            KernfsNodeFlag::empty(),
            node.this_weak(),
        )?;
        perf_event_inherit.set_data(Box::new(CgroupSubsystemData::new("0")))?;

        let perf_event_read = KernfsNode::new_attr(
            "perf_event.read",
            Some(InodeMode::from_bits_truncate(0o755)),
            KernfsNodeFlag::empty(),
            node.this_weak(),
        )?;
        perf_event_read.set_data(Box::new(CgroupSubsystemData::new("0")))?;

        let perf_event_stat = KernfsNode::new_attr(
            "perf_event.stat",
            Some(InodeMode::from_bits_truncate(0o755)),
            KernfsNodeFlag::empty(),
            node.this_weak(),
        )?;
        perf_event_stat.set_data(Box::new(CgroupSubsystemData::new("0")))?;

        Ok(node)
    }
}

struct PidsSubsystem;

impl PidsSubsystem {
    pub fn new(name: &str, parent: Arc<KernfsNode>) -> Result<Arc<KernfsNode>> {
        let mode = InodeMode::from_bits_truncate(0o555);
        let node = KernfsNode::new_dir(
            name,
            Some(mode),
            KernfsNodeFlag::empty(),
            parent.this_weak(),
        )?;

        let pids_max = KernfsNode::new_attr(
            "pids.max",
            Some(InodeMode::from_bits_truncate(0o755)),
            KernfsNodeFlag::empty(),
            node.this_weak(),
        )?;
        pids_max.set_data(Box::new(CgroupSubsystemData::new("max")))?;

        let pids_current = KernfsNode::new_attr(
            "pids.current",
            Some(InodeMode::from_bits_truncate(0o755)),
            KernfsNodeFlag::empty(),
            node.this_weak(),
        )?;
        pids_current.set_data(Box::new(CgroupSubsystemData::new("0")))?;

        Ok(node)
    }
}

struct SystemdSubsystem;

impl SystemdSubsystem {
    pub fn new(name: &str, parent: Arc<KernfsNode>) -> Result<Arc<KernfsNode>> {
        let mode = InodeMode::from_bits_truncate(0o555);
        let node = KernfsNode::new_dir(
            name,
            Some(mode),
            KernfsNodeFlag::empty(),
            parent.this_weak(),
        )?;

        let cgroup_clone_children = KernfsNode::new_attr(
            "cgroup.clone_children",
            Some(InodeMode::from_bits_truncate(0o755)),
            KernfsNodeFlag::empty(),
            node.this_weak(),
        )?;
        cgroup_clone_children.set_data(Box::new(CgroupSubsystemData::new("0")))?;

        let cgroup_procs = KernfsNode::new_attr(
            "cgroup.procs",
            Some(InodeMode::from_bits_truncate(0o755)),
            KernfsNodeFlag::empty(),
            node.this_weak(),
        )?;
        cgroup_procs.set_data(Box::new(CgroupSubsystemData::new("")))?;

        let notify_on_release = KernfsNode::new_attr(
            "notify_on_release",
            Some(InodeMode::from_bits_truncate(0o755)),
            KernfsNodeFlag::empty(),
            node.this_weak(),
        )?;
        notify_on_release.set_data(Box::new(CgroupSubsystemData::new("0")))?;

        let tasks = KernfsNode::new_attr(
            "tasks",
            Some(InodeMode::from_bits_truncate(0o755)),
            KernfsNodeFlag::empty(),
            node.this_weak(),
        )?;
        tasks.set_data(Box::new(CgroupSubsystemData::new("")))?;

        Ok(node)
    }
}

struct RdmaSubsystem;

impl RdmaSubsystem {
    pub fn new(name: &str, parent: Arc<KernfsNode>) -> Result<Arc<KernfsNode>> {
        let mode = InodeMode::from_bits_truncate(0o555);
        let node = KernfsNode::new_dir(
            name,
            Some(mode),
            KernfsNodeFlag::empty(),
            parent.this_weak(),
        )?;

        let rdma_max = KernfsNode::new_attr(
            "rdma.max",
            Some(InodeMode::from_bits_truncate(0o755)),
            KernfsNodeFlag::empty(),
            node.this_weak(),
        )?;
        rdma_max.set_data(Box::new(CgroupSubsystemData::new("0")))?;

        let rdma_current = KernfsNode::new_attr(
            "rdma.current",
            Some(InodeMode::from_bits_truncate(0o755)),
            KernfsNodeFlag::empty(),
            node.this_weak(),
        )?;
        rdma_current.set_data(Box::new(CgroupSubsystemData::new("0")))?;

        Ok(node)
    }
}

struct BlkioSubsystem;

impl BlkioSubsystem {
    pub fn new(name: &str, parent: Arc<KernfsNode>) -> Result<Arc<KernfsNode>> {
        let mode = InodeMode::from_bits_truncate(0o555);
        let node = KernfsNode::new_dir(
            name,
            Some(mode),
            KernfsNodeFlag::empty(),
            parent.this_weak(),
        )?;

        let blkio_reset_stats = KernfsNode::new_attr(
            "blkio.reset_stats",
            Some(InodeMode::from_bits_truncate(0o755)),
            KernfsNodeFlag::empty(),
            node.this_weak(),
        )?;
        blkio_reset_stats.set_data(Box::new(CgroupSubsystemData::new("0")))?;

        let blkio_sectors = KernfsNode::new_attr(
            "blkio.sectors",
            Some(InodeMode::from_bits_truncate(0o755)),
            KernfsNodeFlag::empty(),
            node.this_weak(),
        )?;
        blkio_sectors.set_data(Box::new(CgroupSubsystemData::new("0")))?;

        let blkio_io_service_bytes = KernfsNode::new_attr(
            "blkio.io_service_bytes",
            Some(InodeMode::from_bits_truncate(0o755)),
            KernfsNodeFlag::empty(),
            node.this_weak(),
        )?;
        blkio_io_service_bytes.set_data(Box::new(CgroupSubsystemData::new("0")))?;

        let blkio_io_serviced = KernfsNode::new_attr(
            "blkio.io_serviced",
            Some(InodeMode::from_bits_truncate(0o755)),
            KernfsNodeFlag::empty(),
            node.this_weak(),
        )?;
        blkio_io_serviced.set_data(Box::new(CgroupSubsystemData::new("0")))?;

        let blkio_io_service_time = KernfsNode::new_attr(
            "blkio.io_service_time",
            Some(InodeMode::from_bits_truncate(0o755)),
            KernfsNodeFlag::empty(),
            node.this_weak(),
        )?;
        blkio_io_service_time.set_data(Box::new(CgroupSubsystemData::new("0")))?;

        let blkio_io_wait_time = KernfsNode::new_attr(
            "blkio.io_wait_time",
            Some(InodeMode::from_bits_truncate(0o755)),
            KernfsNodeFlag::empty(),
            node.this_weak(),
        )?;
        blkio_io_wait_time.set_data(Box::new(CgroupSubsystemData::new("0")))?;

        let blkio_io_merged = KernfsNode::new_attr(
            "blkio.io_merged",
            Some(InodeMode::from_bits_truncate(0o755)),
            KernfsNodeFlag::empty(),
            node.this_weak(),
        )?;
        blkio_io_merged.set_data(Box::new(CgroupSubsystemData::new("0")))?;

        let blkio_time = KernfsNode::new_attr(
            "blkio.time",
            Some(InodeMode::from_bits_truncate(0o755)),
            KernfsNodeFlag::empty(),
            node.this_weak(),
        )?;
        blkio_time.set_data(Box::new(CgroupSubsystemData::new("0")))?;

        let blkio_delay = KernfsNode::new_attr(
            "blkio.delay",
            Some(InodeMode::from_bits_truncate(0o755)),
            KernfsNodeFlag::empty(),
            node.this_weak(),
        )?;
        blkio_delay.set_data(Box::new(CgroupSubsystemData::new("0")))?;

        let blkio_throttle_io_service_bytes = KernfsNode::new_attr(
            "blkio.throttle.io_service_bytes",
            Some(InodeMode::from_bits_truncate(0o755)),
            KernfsNodeFlag::empty(),
            node.this_weak(),
        )?;
        blkio_throttle_io_service_bytes.set_data(Box::new(CgroupSubsystemData::new("0")))?;

        let blkio_throttle_io_serviced = KernfsNode::new_attr(
            "blkio.throttle.io_serviced",
            Some(InodeMode::from_bits_truncate(0o755)),
            KernfsNodeFlag::empty(),
            node.this_weak(),
        )?;
        blkio_throttle_io_serviced.set_data(Box::new(CgroupSubsystemData::new("0")))?;

        let blkio_throttle_read_bps_device = KernfsNode::new_attr(
            "blkio.throttle.read_bps_device",
            Some(InodeMode::from_bits_truncate(0o755)),
            KernfsNodeFlag::empty(),
            node.this_weak(),
        )?;
        blkio_throttle_read_bps_device.set_data(Box::new(CgroupSubsystemData::new("0")))?;

        let blkio_throttle_write_bps_device = KernfsNode::new_attr(
            "blkio.throttle.write_bps_device",
            Some(InodeMode::from_bits_truncate(0o755)),
            KernfsNodeFlag::empty(),
            node.this_weak(),
        )?;
        blkio_throttle_write_bps_device.set_data(Box::new(CgroupSubsystemData::new("0")))?;

        let blkio_throttle_read_iops_device = KernfsNode::new_attr(
            "blkio.throttle.read_iops_device",
            Some(InodeMode::from_bits_truncate(0o755)),
            KernfsNodeFlag::empty(),
            node.this_weak(),
        )?;
        blkio_throttle_read_iops_device.set_data(Box::new(CgroupSubsystemData::new("0")))?;

        let blkio_throttle_write_iops_device = KernfsNode::new_attr(
            "blkio.throttle.write_iops_device",
            Some(InodeMode::from_bits_truncate(0o755)),
            KernfsNodeFlag::empty(),
            node.this_weak(),
        )?;
        blkio_throttle_write_iops_device.set_data(Box::new(CgroupSubsystemData::new("0")))?;

        Ok(node)
    }
}