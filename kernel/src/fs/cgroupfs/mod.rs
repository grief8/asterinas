// SPDX-License-Identifier: MPL-2.0

/// A toy pseudo filesystem that is working as cgroupfs.
/// It takes effect if you mount it to `/cgroup`.
use core::sync::atomic::{AtomicU64, Ordering};

use ostd::mm::VmWriter;

use crate::{
    fs::{
        kernfs::{DataProvider, PseudoFileSystem, KernfsNode, KernfsNodeFlag},
        utils::{FileSystem, FsFlags, Inode, SuperBlock, NAME_MAX},
    },
    prelude::*,
};

/// ToyCgroupFS filesystem.
/// Magic number.
const CGROUPFS_MAGIC: u64 = 0x9fa1;
/// Root Inode ID.
const CGROUPFS_ROOT_INO: u64 = 1;
/// Block size.
const BLOCK_SIZE: usize = 1024;

pub struct ToyCgroupFS {
    sb: SuperBlock,
    root: Arc<dyn Inode>,
    inode_allocator: AtomicU64,
    this: Weak<Self>,
}

impl ToyCgroupFS {
    pub fn new() -> Arc<Self> {
        Arc::new_cyclic(|weak_fs| Self {
            sb: SuperBlock::new(CGROUPFS_MAGIC, BLOCK_SIZE, NAME_MAX),
            root: CgroupfsRoot::new_inode(weak_fs.clone()),
            inode_allocator: AtomicU64::new(CGROUPFS_ROOT_INO + 1),
            this: weak_fs.clone(),
        })
    }
}

impl PseudoFileSystem for ToyCgroupFS {
    fn alloc_id(&self) -> u64 {
        self.inode_allocator.fetch_add(1, Ordering::SeqCst)
    }

    fn init(&self) -> Result<()> {
        // /sys/fs/cgroup/cpuset/
        // /sys/fs/cgroup/net_cls,net_prio/
        // /sys/fs/cgroup/perf_event/
        // /sys/fs/cgroup/hugetlb/
        // /sys/fs/cgroup/freezer/
        // /sys/fs/cgroup/cpu,cpuacct/user.slice/
        // /sys/fs/cgroup/memory/user.slice/
        // /sys/fs/cgroup/pids/user.slice/
        // /sys/fs/cgroup/systemd/user.slice/
        // /sys/fs/cgroup/blkio/user.slice/
        // /sys/fs/cgroup/devices/user.slice/
        let root = self.root_inode();
        let root = root.downcast_ref::<KernfsNode>().unwrap();
        KernfsNode::new_dir("cpuset", None, KernfsNodeFlag::empty(), root.this_weak())?;
        KernfsNode::new_dir("net_cls,net_prio", None, KernfsNodeFlag::empty(), root.this_weak())?;
        KernfsNode::new_dir("perf_event", None, KernfsNodeFlag::empty(), root.this_weak())?;
        KernfsNode::new_dir("hugetlb", None, KernfsNodeFlag::empty(), root.this_weak())?;
        KernfsNode::new_dir("freezer", None, KernfsNodeFlag::empty(), root.this_weak())?;
        let cc = KernfsNode::new_dir("cpu,cpuacct", None, KernfsNodeFlag::empty(), root.this_weak())?;
        KernfsNode::new_dir("user.slice", None, KernfsNodeFlag::empty(), cc.this_weak())?;
        let mc = KernfsNode::new_dir("memory", None, KernfsNodeFlag::empty(), root.this_weak())?;
        KernfsNode::new_dir("user.slice", None, KernfsNodeFlag::empty(), mc.this_weak())?;
        let pc = KernfsNode::new_dir("pids", None, KernfsNodeFlag::empty(), root.this_weak())?;
        KernfsNode::new_dir("user.slice", None, KernfsNodeFlag::empty(), pc.this_weak())?;
        let sc = KernfsNode::new_dir("systemd", None, KernfsNodeFlag::empty(), root.this_weak())?;
        KernfsNode::new_dir("user.slice", None, KernfsNodeFlag::empty(), sc.this_weak())?;
        let bc = KernfsNode::new_dir("blkio", None, KernfsNodeFlag::empty(), root.this_weak())?;
        KernfsNode::new_dir("user.slice", None, KernfsNodeFlag::empty(), bc.this_weak())?;
        let dc = KernfsNode::new_dir("devices", None, KernfsNodeFlag::empty(), root.this_weak())?;
        KernfsNode::new_dir("user.slice", None, KernfsNodeFlag::empty(), dc.this_weak())?;
        Ok(())
    }

    fn fs(&self) -> Arc<dyn FileSystem> {
        self.this.upgrade().unwrap()
    }
}

impl FileSystem for ToyCgroupFS {
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
    pub fn new_inode(fs: Weak<ToyCgroupFS>) -> Arc<dyn Inode> {
        KernfsNode::new_root("cgroupfs", fs, CGROUPFS_ROOT_INO, BLOCK_SIZE)
    }
}
