// SPDX-License-Identifier: MPL-2.0

use core::net::Ipv4Addr;

use super::RawSocketOption;
use crate::{
    impl_ip_options,
    impl_raw_socket_option,
    net::socket::ip::stream::options::{Congestion, KeepAlive, KeepIdle, MaxSegment, NoDelay, WindowClamp},
    prelude::*,
    util::net::options::SocketOption,
};

/// Sock options for IP socket.
///
/// The raw definition is from https://elixir.bootlin.com/linux/v6.0.9/source/include/uapi/linux/in.h#L116
#[repr(i32)]
#[derive(Debug, Clone, Copy, TryFromInt)]
#[allow(non_camel_case_types)]
#[allow(clippy::upper_case_acronyms)]
pub enum CIpOptionName {
    IP_TTL = 2,          /* IP time-to-live */
    IP_TOS = 1,          /* IP type-of-service */
    IP_MULTICAST_IF = 11, /* IP multicast interface */
    IP_MULTICAST_TTL = 33, /* IP multicast TTL */
    IP_MULTICAST_LOOP = 34, /* IP multicast loopback */
    IP_ADD_MEMBERSHIP = 35, /* Add an IP group membership */
    IP_DROP_MEMBERSHIP = 36, /* Drop an IP group membership */
}

pub fn new_ip_option(name: i32) -> Result<Box<dyn RawSocketOption>> {
    let name = CIpOptionName::try_from(name)?;
    match name {
        CIpOptionName::IP_TTL => Ok(Box::new(IpTtl::new())),
        CIpOptionName::IP_TOS => Ok(Box::new(IpTos::new())),
        CIpOptionName::IP_MULTICAST_IF => Ok(Box::new(IpMulticastIf::new())),
        CIpOptionName::IP_MULTICAST_TTL => Ok(Box::new(IpMulticastTtl::new())),
        CIpOptionName::IP_MULTICAST_LOOP => Ok(Box::new(IpMulticastLoop::new())),
        CIpOptionName::IP_ADD_MEMBERSHIP => Ok(Box::new(IpAddMembership::new())),
        CIpOptionName::IP_DROP_MEMBERSHIP => Ok(Box::new(IpDropMembership::new())),
        _ => todo!(),
    }
}

impl_ip_options!(
    pub struct IpTtl(u32);
    pub struct IpTos(u32);
    pub struct IpMulticastIf(u32);
    pub struct IpMulticastTtl(u32);
    pub struct IpMulticastLoop(bool);
    pub struct IpAddMembership(u32);
    pub struct IpDropMembership(u32);
);

// IP options
impl_raw_socket_option!(IpTtl);
impl_raw_socket_option!(IpTos);
impl_raw_socket_option!(IpMulticastIf);
impl_raw_socket_option!(IpMulticastTtl);
impl_raw_socket_option!(IpMulticastLoop);
impl_raw_socket_option!(IpAddMembership);
impl_raw_socket_option!(IpDropMembership);

// #[derive(Debug)]
// pub struct IpMembershipOption {
//     pub interface_index: u32,
//     pub group_address: Ipv4Addr,
// }

// impl IpMembershipOption {
//     pub fn new(interface_index: u32, group_address: Ipv4Addr) -> Self {
//         Self {
//             interface_index,
//             group_address,
//         }
//     }
// }

// impl Default for IpMembershipOption {
//     fn default() -> Self {
//         Self {
//             interface_index: 0,
//             group_address: Ipv4Addr::new(224, 0, 0, 1),
//         }
//     }
// }

