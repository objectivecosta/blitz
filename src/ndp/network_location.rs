use std::net::Ipv6Addr;

use pnet::util::MacAddr;

#[derive(Clone, Copy)]
pub struct V6NetworkLocation {
    pub ipv6: Ipv6Addr,
    pub hw: MacAddr
}