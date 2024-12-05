// SPDX-License-Identifier: MPL-2.0

use super::*;
/// Represents the inode at `/proc/self/ns`.
use crate::fs::procfs::template::{ProcSymBuilder, SymOps};
use crate::{
    fs::{
        procfs::template::{DirOps, ProcDir, ProcDirBuilder},
        utils::{DirEntryVecExt, Inode},
    },
    prelude::*,
};

pub struct NsDirOps;

impl NsDirOps {
    pub fn new_inode(parent: Weak<dyn Inode>) -> Arc<dyn Inode> {
        ProcDirBuilder::new(Self).parent(parent).build().unwrap()
    }

    fn create_symlink(&self, parent: Weak<dyn Inode>, target: &str) -> Arc<dyn Inode> {
        SymlinkOps::new_inode(target, parent)
    }
}

struct SymlinkOps {
    target: String,
}

impl SymlinkOps {
    pub fn new_inode(target: &str, parent: Weak<dyn Inode>) -> Arc<dyn Inode> {
        ProcSymBuilder::new(Self {
            target: target.to_string(),
        })
        .parent(parent)
        .build()
        .unwrap()
    }
}

impl SymOps for SymlinkOps {
    fn read_link(&self) -> Result<String> {
        Ok(self.target.clone())
    }
}

impl DirOps for NsDirOps {
    fn lookup_child(&self, this_ptr: Weak<dyn Inode>, name: &str) -> Result<Arc<dyn Inode>> {
        let inode = match name {
            "cgroup" => self.create_symlink(this_ptr.clone(), "'cgroup:[4026531835]'"),
            "ipc" => self.create_symlink(this_ptr.clone(), "ipc:[4026531839]"),
            "mnt" => self.create_symlink(this_ptr.clone(), "mnt:[4026531840]"),
            "net" => self.create_symlink(this_ptr.clone(), "net:[4026531992]"),
            "pid" => self.create_symlink(this_ptr.clone(), "pid:[4026531836]"),
            "pid_for_children" => self.create_symlink(this_ptr.clone(), "pid:[4026531836]"),
            "time" => self.create_symlink(this_ptr.clone(), "time:[4026531834]"),
            "time_for_children" => self.create_symlink(this_ptr.clone(), "time:[4026531834]"),
            "user" => self.create_symlink(this_ptr.clone(), "'user:[4026531837]'"),
            "uts" => self.create_symlink(this_ptr.clone(), "uts:[4026531838]"),
            _ => return_errno!(Errno::ENOENT),
        };
        Ok(inode)
    }

    fn populate_children(&self, this_ptr: Weak<dyn Inode>) {
        let this = {
            let this = this_ptr.upgrade().unwrap();
            this.downcast_ref::<ProcDir<NsDirOps>>().unwrap().this()
        };
        let mut cached_children = this.cached_children().write();
        cached_children.put_entry_if_not_found("cgroup", || {
            self.create_symlink(this_ptr.clone(), "cgroup:[4026531835]")
        });
        cached_children.put_entry_if_not_found("ipc", || {
            self.create_symlink(this_ptr.clone(), "ipc:[4026531839]")
        });
        cached_children.put_entry_if_not_found("mnt", || {
            self.create_symlink(this_ptr.clone(), "mnt:[4026531840]")
        });
        cached_children.put_entry_if_not_found("net", || {
            self.create_symlink(this_ptr.clone(), "net:[4026531992]")
        });
        cached_children.put_entry_if_not_found("pid", || {
            self.create_symlink(this_ptr.clone(), "pid:[4026531836]")
        });
        cached_children.put_entry_if_not_found("pid_for_children", || {
            self.create_symlink(this_ptr.clone(), "pid:[4026531836]")
        });
        cached_children.put_entry_if_not_found("time", || {
            self.create_symlink(this_ptr.clone(), "time:[4026531834]")
        });
        cached_children.put_entry_if_not_found("time_for_children", || {
            self.create_symlink(this_ptr.clone(), "time:[4026531834]")
        });
        cached_children.put_entry_if_not_found("user", || {
            self.create_symlink(this_ptr.clone(), "'user:[4026531837]'")
        });
        cached_children.put_entry_if_not_found("uts", || {
            self.create_symlink(this_ptr.clone(), "uts:[4026531838]")
        });
    }
}
