use std::net::Ipv4Addr;

use pnet::util::MacAddr;

#[derive(Clone, Copy)]
pub struct NetworkLocation {
    pub ipv6: Ipv6Addr,
    pub hw: MacAddr
}