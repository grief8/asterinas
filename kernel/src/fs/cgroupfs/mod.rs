// SPDX-License-Identifier: MPL-2.0
use alloc::sync::{Arc, Weak};
/// A pseudo filesystem that functions as cgroupfs.
/// It takes effect if you mount it to `/sys/fs/cgroup`.
use core::{
    sync::atomic::{AtomicU64, Ordering},
    time::Duration,
};

use aster_rights::Full;
use inherit_methods_macro::inherit_methods;
use ostd::mm::VmWriter;

use super::{
    kernfs::PseudoNode,
    utils::{InodeMode, InodeType, IoctlCmd, Metadata},
};
use crate::{
    fs::{
        kernfs::{DataProvider, KernfsNode, KernfsNodeFlag, PseudoFileSystem},
        utils::{FileSystem, FsFlags, Inode, SuperBlock, NAME_MAX},
    },
    prelude::*,
    process::{Gid, Uid},
    vm::vmo::Vmo,
};

/// Constants
const CGROUPFS_MAGIC: u64 = 0x27e0eb;
const CGROUPFS_ROOT_INO: u64 = 1;
const BLOCK_SIZE: usize = 1024;

/// CgroupFS filesystem.
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

    pub fn get_cgroup_config() -> Vec<(&'static str, Vec<(&'static str, &'static str)>)> {
        // Define all subsystems and their attributes with initial values
        vec![
            (
                "cpu",
                vec![
                    ("cgroup.clone_children", "0"),
                    ("cpuacct.stat", "user 0\nsystem 0"),
                    ("cpuacct.usage", "0"),
                    ("cpuacct.usage_percpu", "0"),
                    ("cpuacct.usage_percpu_sys", "0"),
                    ("cpuacct.usage_percpu_user", "0"),
                    ("cpu.cfs_period_us", "100000"),
                    ("cpu.cfs_quota_us", "-1"),
                    ("cpu.cfs_burst_us", "0"),
                    ("cpu.shares", "1024"),
                    ("cpu.stat", "nr_periods 0\nnr_throttled 0\nthrottled_time 0"),
                    ("notify_on_release", "0"),
                    ("tasks", ""),
                ],
            ),
            (
                "memory",
                vec![
                    ("memory.usage_in_bytes", "0"),
                    ("memory.limit_in_bytes", "9223372036854771712"),
                    ("memory.failcnt", "0"),
                    ("memory.max_usage_in_bytes", "0"),
                    ("memory.soft_limit_in_bytes", "9223372036854771712"),
                    ("memory.kmem.slabinfo", ""),
                    ("memory.kmem.tcp.slabinfo", ""),
                    (
                        "memory.oom_control",
                        "oom_kill_disable 0\nunder_oom 0\noom_kill 0\n",
                    ),
                    ("memory.kmem.usage_in_bytes", "0"),
                    ("memory.kmem.limit_in_bytes", "9223372036854771712"),
                    ("memory.kmem.failcnt", "0"),
                    ("memory.kmem.max_usage_in_bytes", "0"),
                    ("memory.kmem.tcp.usage_in_bytes", "0"),
                    ("memory.kmem.tcp.limit_in_bytes", "9223372036854771712"),
                    ("memory.kmem.tcp.failcnt", "0"),
                    ("memory.kmem.tcp.max_usage_in_bytes", "0"),
                ],
            ),
            (
                "cpuset",
                vec![
                    ("cpuset.cpus", "0"),
                    ("cpuset.mems", "0"),
                    ("cpuset.memory_migrate", "0"),
                    ("cpuset.cpu_exclusive", "0"),
                    ("cpuset.mem_exclusive", "0"),
                    ("cpuset.mem_hardwall", "0"),
                    ("cpuset.memory_pressure", "0"),
                    ("cpuset.memory_spread_page", "0"),
                    ("cpuset.memory_spread_slab", "0"),
                    ("cpuset.sched_load_balance", "1"),
                    ("cpuset.sched_relax_domain_level", "-1"),
                    ("cgroup.procs", ""),
                    ("notify_on_release", "0"),
                    ("tasks", ""),
                ],
            ),
            (
                "devices",
                vec![
                    ("devices.allow", ""),
                    ("devices.deny", ""),
                    ("devices.list", ""),
                    ("devices.log_level", ""),
                    ("devices.max_count", ""),
                    ("devices.priority", ""),
                ],
            ),
            (
                "freezer",
                vec![
                    ("freezer.state", "THAWED"),
                    ("freezer.self_freezing", "0"),
                    ("freezer.parent_freezing", "0"),
                ],
            ),
            (
                "hugetlb",
                vec![
                    ("hugetlb.max", "0"),
                    ("hugetlb.current", "0"),
                    ("hugetlb.failcnt", "0"),
                    ("hugetlb.limit_in_bytes", "0"),
                    ("hugetlb.usage_in_bytes", "0"),
                ],
            ),
            (
                "net_cls",
                vec![
                    ("net_cls.classid", "0"),
                    ("net_cls.cgroups", "0"),
                    ("net_cls.mark", "0"),
                ],
            ),
            (
                "perf_event",
                vec![
                    ("perf_event.enable", "0"),
                    ("perf_event.events", "0"),
                    ("perf_event.inherit", "0"),
                    ("perf_event.read", "0"),
                    ("perf_event.stat", "0"),
                ],
            ),
            ("pids", vec![("pids.max", "max"), ("pids.current", "0")]),
            (
                "systemd",
                vec![
                    ("cgroup.clone_children", "0"),
                    ("cgroup.procs", ""),
                    ("notify_on_release", "0"),
                    ("tasks", ""),
                ],
            ),
            ("rdma", vec![("rdma.max", "0"), ("rdma.current", "0")]),
            (
                "blkio",
                vec![
                    ("blkio.reset_stats", "0"),
                    ("blkio.sectors", "0"),
                    ("blkio.io_service_bytes", "0"),
                    ("blkio.io_serviced", "0"),
                    ("blkio.io_service_time", "0"),
                    ("blkio.io_wait_time", "0"),
                    ("blkio.io_merged", "0"),
                    ("blkio.time", "0"),
                    ("blkio.delay", "0"),
                    ("blkio.throttle.io_service_bytes", "0"),
                    ("blkio.throttle.io_serviced", "0"),
                    ("blkio.throttle.read_bps_device", "0"),
                    ("blkio.throttle.write_bps_device", "0"),
                    ("blkio.throttle.read_iops_device", "0"),
                    ("blkio.throttle.write_iops_device", "0"),
                ],
            ),
            (
                "unified",
                vec![
                    ("cgroup.controllers", ""),
                    ("cgroup.subtree_control", ""),
                    ("cgroup.events", ""),
                    ("cgroup.max.depth", "0"),
                    ("cgroup.max.descendants", "0"),
                    ("cgroup.stat", ""),
                    ("cgroup.threads", ""),
                    ("cgroup.type", ""),
                    ("cgroup.freeze", "0"),
                    ("cgroup.kill", "0"),
                    ("cgroup.procs", ""),
                ],
            ),
        ]
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
            .downcast_ref::<CgroupSubsystem>()
            .ok_or_else(|| Error::new(Errno::EINVAL))?;

        for (subsys_name, attributes) in CgroupFS::get_cgroup_config() {
            let subsystem_node =
                CgroupSubsystem::new_subsystem(subsys_name, root.this(), &attributes)?;
            // let container = CgroupSubsystem::new_subsystem("mycontainer", subsystem_node.this(), &attributes)?;
            // Optionally, create symlinks if needed
            if subsys_name == "cpu" {
                CgroupSubsystem::new_link("cpuacct", root.this(), subsystem_node.this())?;
                CgroupSubsystem::new_link("cpu,cpuacct", root.this(), subsystem_node.this())?;
            }
            if subsys_name == "net_cls" {
                CgroupSubsystem::new_link("net_prio", root.this(), subsystem_node.this())?;
                CgroupSubsystem::new_link("net_cls,net_prio", root.this(), subsystem_node.this())?;
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
        CgroupSubsystem::new_root("cgroupfs", fs)
    }
}

/// Generic Cgroup Subsystem
#[derive(Clone)]
pub struct CgroupSubsystem(Arc<KernfsNode>);

impl CgroupSubsystem {
    /// Creates a new subsystem with the given name and attributes.
    pub fn new_subsystem(
        name: &str,
        parent: Arc<CgroupSubsystem>,
        attributes: &[(&str, &str)],
    ) -> Result<Arc<Self>> {
        let node =
            KernfsNode::new_dir(name, None, KernfsNodeFlag::empty(), parent.this_weak_node())?;

        // Create attribute nodes
        for (attr_name, initial_value) in attributes {
            let attr_node = KernfsNode::new_attr(
                attr_name,
                Some(InodeMode::from_bits_truncate(0o644)),
                KernfsNodeFlag::empty(),
                node.this_weak(),
            )?;
            node.insert(attr_name.to_string(), attr_node.this())?;
            attr_node.set_data(Box::new(CgroupSubsystemData::new(initial_value)))?;
        }

        let cgroup_node = Arc::new(Self(node));
        parent.insert(name.to_string(), cgroup_node.clone())?;
        Ok(cgroup_node)
    }

    pub fn new_link(
        name: &str,
        parent: Arc<dyn PseudoNode>,
        target: Arc<dyn PseudoNode>,
    ) -> Result<Arc<Self>> {
        let node = KernfsNode::new_symlink(
            name,
            KernfsNodeFlag::empty(),
            target,
            Arc::downgrade(&parent.clone()),
        )?;
        let symlink = Arc::new(Self(node));
        parent.insert(name.to_string(), symlink.clone())?;
        Ok(symlink)
    }

    pub fn new_root(name: &str, fs: Weak<CgroupFS>) -> Arc<Self> {
        let node = KernfsNode::new_root(name, fs, CGROUPFS_ROOT_INO, BLOCK_SIZE);
        Arc::new(Self(node))
    }

    pub fn this_weak(&self) -> Weak<CgroupSubsystem> {
        Arc::downgrade(&self.this())
    }

    pub fn this_weak_node(&self) -> Weak<KernfsNode> {
        self.0.this_weak()
    }

    pub fn this(&self) -> Arc<Self> {
        Arc::new(self.clone())
    }

    pub fn this_node(&self) -> Arc<KernfsNode> {
        self.0.clone()
    }
}

impl PseudoNode for CgroupSubsystem {
    fn name(&self) -> String {
        self.0.name()
    }

    fn parent(&self) -> Option<Arc<dyn PseudoNode>> {
        self.0.parent()
    }

    fn pseudo_fs(&self) -> Arc<dyn PseudoFileSystem> {
        self.0.pseudo_fs()
    }

    fn generate_ino(&self) -> u64 {
        self.0.generate_ino()
    }

    fn set_data(&self, data: Box<dyn DataProvider>) -> Result<()> {
        self.0.set_data(data)
    }

    fn remove(&self, name: &str) -> Result<()> {
        debug!("CgroupSubsystem remove: {}", name);
        self.0.remove(name)
    }

    fn insert(&self, name: String, node: Arc<dyn Inode>) -> Result<()> {
        self.0.insert(name, node)
    }

    fn get_children(&self) -> Option<BTreeMap<String, Arc<dyn Inode>>> {
        self.0.get_children()
    }
}

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
        _poller: Option<&mut crate::process::signal::PollHandle>,
    ) -> crate::events::IoEvents;
    fn is_seekable(&self) -> bool;
    fn extension(&self) -> Option<&super::utils::Extension>;
    fn mknod(
        &self,
        name: &str,
        mode: super::utils::InodeMode,
        type_: super::utils::MknodType,
    ) -> Result<Arc<dyn Inode>>;

    fn lookup(&self, _name: &str) -> Result<Arc<dyn Inode>>;
    fn create(&self, name: &str, type_: InodeType, _mode: InodeMode) -> Result<Arc<dyn Inode>> {
        if self.0.type_() != InodeType::Dir {
            return_errno!(Errno::ENOTDIR);
        }
        if self.0.lookup(name).is_ok() {
            return_errno!(Errno::EEXIST);
        }
        let new_node = match type_ {
            InodeType::Dir => {
                let config = CgroupFS::get_cgroup_config();
                let attributes = config
                    .iter()
                    .find(|(subsys_name, _)| *subsys_name == self.0.name());
                CgroupSubsystem::new_subsystem(name, self.this(), &attributes.unwrap().1)?
            }
            _ => return_errno!(Errno::EINVAL),
        };
        Ok(new_node)
    }
}

/// Data provider for cgroup subsystem attributes.
struct CgroupSubsystemData {
    data: Vec<u8>,
}

impl CgroupSubsystemData {
    pub fn new(data: &str) -> Self {
        let mut data_vec = data.to_string();
        if !data_vec.ends_with('\n') {
            data_vec.push('\n');
        }
        Self {
            data: data_vec.into_bytes(),
        }
    }
}

impl DataProvider for CgroupSubsystemData {
    fn read_at(&self, writer: &mut VmWriter, offset: usize) -> Result<usize> {
        let start = if offset < self.data.len() {
            offset
        } else {
            self.data.len()
        };
        let end = if offset + writer.avail() > self.data.len() {
            self.data.len()
        } else {
            offset + writer.avail()
        };
        let len = end - start;
        if len > 0 {
            writer.write_fallible(&mut (&self.data[start..end]).into())?;
        }
        Ok(len)
    }

    fn write_at(&mut self, reader: &mut VmReader, offset: usize) -> Result<usize> {
        let write_len = reader.remain();
        let end = offset + write_len;
        if self.data.len() < end {
            self.data.resize(end, 0);
        }
        {
            let slice = &mut self.data[offset..end];
            let mut writer = VmWriter::from(slice);
            let actual_written = reader.read_fallible(&mut writer)?;
            if actual_written != write_len {
                return_errno!(Errno::EINVAL);
            }
        }
        Ok(write_len)
    }
}
