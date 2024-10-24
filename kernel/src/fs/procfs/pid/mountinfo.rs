// SPDX-License-Identifier: MPL-2.0

use alloc::format;

use crate::{
    fs::{
        procfs::template::{FileOps, ProcFileBuilder},
        utils::Inode,
    },
    prelude::*,
    Process,
};

/// Represents the inode at `/proc/[pid]/mountinfo`.
/// See https://www.kernel.org/doc/Documentation/filesystems/proc.txt for details.
/// FIXME: Some fields are not implemented yet.
///
/// Fields:
/// - Mount ID: Unique identifier of the mount (mount id).
/// - Parent ID: The mount ID of the parent mount (or of self for the root of this mount namespace).
/// - Major:Minor: The device numbers of the device.
/// - Root: The pathname of the directory in the filesystem which forms the root of this mount.
/// - Mount Point: The pathname of the mount point relative to the process's root directory.
/// - Mount Options: Per-mount options.
/// - Optional Fields: Zero or more fields of the form "tag[:value]".
/// - FSType: The type of filesystem, such as ext3 or nfs.
/// - Source: The source of the mount.
/// - Super Options: Per-superblock options.
pub struct MountInfoFileOps(Arc<Process>);

impl MountInfoFileOps {
    pub fn new_inode(process_ref: Arc<Process>, parent: Weak<dyn Inode>) -> Arc<dyn Inode> {
        ProcFileBuilder::new(Self(process_ref))
            .parent(parent)
            .build()
            .unwrap()
    }
}

impl FileOps for MountInfoFileOps {
    fn data(&self) -> Result<Vec<u8>> {
        let mount_entries = vec![
            (
                1,
                0,
                "0:1",
                "/",
                "/",
                "rw,nosuid,nodev,noexec,relatime",
                "-",
                "sysfs",
                "sysfs",
                "rw,nosuid,nodev,noexec,relatime",
            ),
            (
                2,
                1,
                "0:2",
                "/proc",
                "/proc",
                "rw,nosuid,nodev,noexec,relatime",
                "-",
                "proc",
                "proc",
                "rw,nosuid,nodev,noexec,relatime",
            ),
            (
                3,
                1,
                "0:3",
                "/dev",
                "/dev",
                "rw,nosuid,noexec,relatime,size=262792576k,nr_inodes=65698144,mode=755",
                "-",
                "devtmpfs",
                "devtmpfs",
                "rw,nosuid,noexec,relatime",
            ),
            (
                4,
                1,
                "0:4",
                "/dev/pts",
                "/dev/pts",
                "rw,nosuid,noexec,relatime,gid=5,mode=620,ptmxmode=000",
                "-",
                "devpts",
                "devpts",
                "rw,nosuid,noexec,relatime,gid=5,mode=620,ptmxmode=000",
            ),
            (
                5,
                1,
                "0:5",
                "/run",
                "/run",
                "rw,nosuid,nodev,noexec,relatime,size=52570144k,mode=755",
                "-",
                "tmpfs",
                "tmpfs",
                "rw,nosuid,nodev,noexec,relatime,size=52570144k,mode=755",
            ),
            (
                6,
                1,
                "8:1",
                "/",
                "/",
                "rw,relatime",
                "-",
                "ext4",
                "/dev/mapper/ubuntu--vg-ubuntu--lv",
                "rw,relatime",
            ),
            (
                7,
                1,
                "0:6",
                "/sys/kernel/security",
                "/sys/kernel/security",
                "rw,nosuid,nodev,noexec,relatime",
                "-",
                "securityfs",
                "securityfs",
                "rw,nosuid,nodev,noexec,relatime",
            ),
            (
                34,
                24,
                "0:29",
                "/",
                "/sys/fs/cgroup",
                "ro,nosuid,nodev,noexec,shared:9",
                "-",
                "tmpfs",
                "tmpfs",
                "ro,mode=755",
            ),
            (
                35,
                34,
                "0:30",
                "/",
                "/sys/fs/cgroup/unified",
                "rw,nosuid,nodev,noexec,relatime,shared:10",
                "-",
                "cgroup2",
                "cgroup2",
                "rw",
            ),
            (
                36,
                34,
                "0:31",
                "/",
                "/sys/fs/cgroup/systemd",
                "rw,nosuid,nodev,noexec,relatime,shared:11",
                "-",
                "cgroup",
                "cgroup",
                "rw,xattr,name=systemd",
            ),
            (
                37,
                24,
                "0:32",
                "/",
                "/sys/fs/pstore",
                "rw,nosuid,nodev,noexec,relatime,shared:12",
                "-",
                "pstore",
                "pstore",
                "rw",
            ),
            (
                38,
                24,
                "0:33",
                "/",
                "/sys/firmware/efi/efivars",
                "rw,nosuid,nodev,noexec,relatime,shared:13",
                "-",
                "efivarfs",
                "efivarfs",
                "rw",
            ),
            (
                39,
                24,
                "0:34",
                "/",
                "/sys/fs/bpf",
                "rw,nosuid,nodev,noexec,relatime,shared:14",
                "-",
                "bpf",
                "none",
                "rw,mode=700",
            ),
            (
                40,
                34,
                "0:35",
                "/",
                "/sys/fs/cgroup/devices",
                "rw,nosuid,nodev,noexec,relatime,shared:16",
                "-",
                "cgroup",
                "cgroup",
                "rw,devices",
            ),
            (
                41,
                34,
                "0:36",
                "/",
                "/sys/fs/cgroup/rdma",
                "rw,nosuid,nodev,noexec,relatime,shared:17",
                "-",
                "cgroup",
                "cgroup",
                "rw,rdma",
            ),
            (
                42,
                34,
                "0:37",
                "/",
                "/sys/fs/cgroup/cpu,cpuacct",
                "rw,nosuid,nodev,noexec,relatime,shared:18",
                "-",
                "cgroup",
                "cgroup",
                "rw,cpu,cpuacct",
            ),
            (
                43,
                34,
                "0:38",
                "/",
                "/sys/fs/cgroup/cpuset",
                "rw,nosuid,nodev,noexec,relatime,shared:19",
                "-",
                "cgroup",
                "cgroup",
                "rw,cpuset,clone_children",
            ),
            (
                44,
                34,
                "0:39",
                "/",
                "/sys/fs/cgroup/blkio",
                "rw,nosuid,nodev,noexec,relatime,shared:20",
                "-",
                "cgroup",
                "cgroup",
                "rw,blkio",
            ),
            (
                45,
                34,
                "0:40",
                "/",
                "/sys/fs/cgroup/perf_event",
                "rw,nosuid,nodev,noexec,relatime,shared:21",
                "-",
                "cgroup",
                "cgroup",
                "rw,perf_event",
            ),
            (
                46,
                34,
                "0:41",
                "/",
                "/sys/fs/cgroup/hugetlb",
                "rw,nosuid,nodev,noexec,relatime,shared:22",
                "-",
                "cgroup",
                "cgroup",
                "rw,hugetlb",
            ),
            (
                47,
                34,
                "0:42",
                "/",
                "/sys/fs/cgroup/memory",
                "rw,nosuid,nodev,noexec,relatime,shared:23",
                "-",
                "cgroup",
                "cgroup",
                "rw,memory",
            ),
            (
                48,
                34,
                "0:43",
                "/",
                "/sys/fs/cgroup/pids",
                "rw,nosuid,nodev,noexec,relatime,shared:24",
                "-",
                "cgroup",
                "cgroup",
                "rw,pids",
            ),
            (
                49,
                34,
                "0:44",
                "/",
                "/sys/fs/cgroup/net_cls,net_prio",
                "rw,nosuid,nodev,noexec,relatime,shared:25",
                "-",
                "cgroup",
                "cgroup",
                "rw,net_cls,net_prio",
            ),
            (
                50,
                34,
                "0:45",
                "/",
                "/sys/fs/cgroup/freezer",
                "rw,nosuid,nodev,noexec,relatime,shared:26",
                "-",
                "cgroup",
                "cgroup",
                "rw,freezer",
            ),
        ];

        let mountinfo_output: String = mount_entries
            .iter()
            .map(|entry| {
                format!(
                    "{} {} {} {} {} {} {} {} {} {}\n",
                    entry.0, // Mount ID
                    entry.1, // Parent ID
                    entry.2, // Major:Minor
                    entry.3, // Root
                    entry.4, // Mount Point
                    entry.5, // Mount Options
                    entry.6, // Optional Fields
                    entry.7, // FSType
                    entry.8, // Source
                    entry.9  // Super Options
                )
            })
            .collect();

        Ok(mountinfo_output.into_bytes())
    }
}
