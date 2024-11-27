// SPDX-License-Identifier: MPL-2.0

#![allow(non_camel_case_types)]

use super::process_vm::{INIT_STACK_SIZE, USER_HEAP_SIZE_LIMIT};
use crate::prelude::*;

pub struct ResourceLimits {
    rlimits: [RLimit64; RLIMIT_COUNT],
}

impl ResourceLimits {
    pub fn get_rlimit(&self, resource: ResourceType) -> &RLimit64 {
        &self.rlimits[resource as usize]
    }

    pub fn get_rlimit_mut(&mut self, resource: ResourceType) -> &mut RLimit64 {
        &mut self.rlimits[resource as usize]
    }
}

impl Default for ResourceLimits {
    fn default() -> Self {
        let mut rlimits = [RLimit64::default(); RLIMIT_COUNT];

        // RLIMIT_CPU
        rlimits[ResourceType::RLIMIT_CPU as usize] = RLimit64::new(u64::MAX, u64::MAX);

        // RLIMIT_FSIZE
        rlimits[ResourceType::RLIMIT_FSIZE as usize] = RLimit64::new(u64::MAX, u64::MAX);

        // RLIMIT_DATA
        rlimits[ResourceType::RLIMIT_DATA as usize] =
            RLimit64::new(USER_HEAP_SIZE_LIMIT as u64, u64::MAX);

        // RLIMIT_STACK
        rlimits[ResourceType::RLIMIT_STACK as usize] =
            RLimit64::new(INIT_STACK_SIZE as u64, u64::MAX);

        // RLIMIT_CORE
        rlimits[ResourceType::RLIMIT_CORE as usize] = RLimit64::new(0, u64::MAX);

        // RLIMIT_RSS
        rlimits[ResourceType::RLIMIT_RSS as usize] = RLimit64::new(u64::MAX, u64::MAX);

        // RLIMIT_NPROC
        rlimits[ResourceType::RLIMIT_NPROC as usize] = RLimit64::new(2053063, 2053063);

        // RLIMIT_NOFILE
        rlimits[ResourceType::RLIMIT_NOFILE as usize] = RLimit64::new(1048576, 1048576);

        // RLIMIT_MEMLOCK
        rlimits[ResourceType::RLIMIT_MEMLOCK as usize] = RLimit64::new(67108864, 67108864);

        // RLIMIT_AS
        rlimits[ResourceType::RLIMIT_AS as usize] = RLimit64::new(u64::MAX, u64::MAX);

        // RLIMIT_LOCKS
        rlimits[ResourceType::RLIMIT_LOCKS as usize] = RLimit64::new(u64::MAX, u64::MAX);

        // RLIMIT_SIGPENDING
        rlimits[ResourceType::RLIMIT_SIGPENDING as usize] = RLimit64::new(2053063, 2053063);

        // RLIMIT_MSGQUEUE
        rlimits[ResourceType::RLIMIT_MSGQUEUE as usize] = RLimit64::new(819200, 819200);

        // RLIMIT_NICE
        rlimits[ResourceType::RLIMIT_NICE as usize] = RLimit64::new(0, 0);

        // RLIMIT_RTPRIO
        rlimits[ResourceType::RLIMIT_RTPRIO as usize] = RLimit64::new(0, 0);

        // RLIMIT_RTTIME
        rlimits[ResourceType::RLIMIT_RTTIME as usize] = RLimit64::new(u64::MAX, u64::MAX);

        ResourceLimits { rlimits }
    }
}

#[repr(u32)]
#[derive(Debug, Clone, Copy, TryFromInt)]
pub enum ResourceType {
    RLIMIT_CPU = 0,
    RLIMIT_FSIZE = 1,
    RLIMIT_DATA = 2,
    RLIMIT_STACK = 3,
    RLIMIT_CORE = 4,
    RLIMIT_RSS = 5,
    RLIMIT_NPROC = 6,
    RLIMIT_NOFILE = 7,
    RLIMIT_MEMLOCK = 8,
    RLIMIT_AS = 9,
    RLIMIT_LOCKS = 10,
    RLIMIT_SIGPENDING = 11,
    RLIMIT_MSGQUEUE = 12,
    RLIMIT_NICE = 13,
    RLIMIT_RTPRIO = 14,
    RLIMIT_RTTIME = 15,
}

pub const RLIMIT_COUNT: usize = 16;

#[derive(Debug, Clone, Copy, Pod)]
#[repr(C)]
pub struct RLimit64 {
    cur: u64,
    max: u64,
}

impl RLimit64 {
    pub fn new(cur: u64, max: u64) -> Self {
        Self { cur, max }
    }

    pub fn get_cur(&self) -> u64 {
        self.cur
    }

    pub fn get_max(&self) -> u64 {
        self.max
    }

    pub fn is_valid(&self) -> bool {
        self.cur <= self.max
    }
}

impl Default for RLimit64 {
    fn default() -> Self {
        Self {
            cur: u64::MAX,
            max: u64::MAX,
        }
    }
}
