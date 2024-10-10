// SPDX-License-Identifier: MPL-2.0
#![allow(unused)]

use core::sync::atomic::{AtomicU64, Ordering};

use inode::KObject;
use kernel::files::{AddressBits, CpuByteOrder};

use super::{
    kernfs::{DataProvider, PseudoFileSystem},
    utils::{FileSystem, FsFlags, SuperBlock, NAME_MAX},
};
use crate::{
    fs::{
        kernfs::{KernfsNode, KernfsNodeFlag},
        utils::Inode,
    },
    prelude::*,
};

pub mod inode;
pub mod kernel;

/// SysFS filesystem.
/// Magic number.
const SYSFS_MAGIC: u64 = 0x9fa0;
/// Root Inode ID.
const SYSFS_ROOT_INO: u64 = 1;
/// Block size.
const BLOCK_SIZE: usize = 1024;

pub struct SysFS {
    sb: SuperBlock,
    root: Arc<dyn Inode>,
    inode_allocator: AtomicU64,
    this: Weak<Self>,
}

impl SysFS {
    pub fn new() -> Arc<Self> {
        Arc::new_cyclic(|weak_fs| Self {
            sb: SuperBlock::new(SYSFS_MAGIC, BLOCK_SIZE, NAME_MAX),
            root: SysfsRoot::new_inode(weak_fs.clone()),
            inode_allocator: AtomicU64::new(SYSFS_ROOT_INO + 1),
            this: weak_fs.clone(),
        })
    }

    pub fn create_file(
        name: &str,
        mode: u16,
        parent: Arc<KObject>,
        attribute: Box<dyn DataProvider>,
    ) -> Result<Arc<KObject>> {
        let attr = KObject::new_attr(name, mode, Some(parent.this_weak()))?;
        attr.set_data(attribute).unwrap();
        Ok(attr)
    }

    pub fn create_kobject(name: &str, mode: u16, parent: Arc<KObject>) -> Result<Arc<KObject>> {
        KObject::new_dir(name, mode, Some(parent.this_weak()))
    }
}

impl PseudoFileSystem for SysFS {
    fn alloc_id(&self) -> u64 {
        self.inode_allocator.fetch_add(1, Ordering::SeqCst)
    }

    fn init(&self) -> Result<()> {
        let root = self.root_inode().downcast_ref::<KObject>().unwrap().this();
        SysFS::create_kobject("block", 0o755, root.clone())?;
        SysFS::create_kobject("bus", 0o755, root.clone())?;
        SysFS::create_kobject("class", 0o755, root.clone())?;
        SysFS::create_kobject("dev", 0o755, root.clone())?;
        SysFS::create_kobject("firmware", 0o755, root.clone())?;
        SysFS::create_kobject("fs", 0o755, root.clone())?;
        SysFS::create_kobject("module", 0o755, root.clone())?;
        SysFS::create_kobject("power", 0o755, root.clone())?;
        let kernel = SysFS::create_kobject("kernel", 0o755, root.clone())?;
        SysFS::create_file("address_bits", 0o644, kernel.clone(), Box::new(AddressBits))?;
        SysFS::create_file(
            "byteorder",
            0o644,
            kernel.clone(),
            Box::new(CpuByteOrder::new()),
        )?;

        Ok(())
    }

    fn fs(&self) -> Arc<dyn FileSystem> {
        self.this.upgrade().unwrap()
    }
}

impl FileSystem for SysFS {
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

/// Represents the inode at `/sys`.
/// Root directory of the sysfs.
pub struct SysfsRoot;

impl SysfsRoot {
    pub fn new_inode(fs: Weak<SysFS>) -> Arc<dyn Inode> {
        KObject::new_root("sysfs", fs, SYSFS_ROOT_INO, BLOCK_SIZE)
    }
}
