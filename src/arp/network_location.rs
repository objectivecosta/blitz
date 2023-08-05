use std::net::Ipv4Addr;

use pnet::util::MacAddr;

#[derive(Clone, Copy)]
pub struct NetworkLocation {
    pub ipv4: Ipv4Addr,
    pub hw: MacAddr
}