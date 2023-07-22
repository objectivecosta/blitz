use std::net::{SocketAddr, SocketAddrV4};

use async_trait::async_trait;
use dns_lookup::getnameinfo;
use pnet::packet::ipv4::Ipv4Packet;
use tokio::{runtime::Builder, task};

#[async_trait]
pub trait GetNameAddr {
    async fn get_from_packet(&self, packet: &Ipv4Packet) -> String;
}

pub struct GetNameAddrImpl {}

impl GetNameAddrImpl {}

#[async_trait]
impl GetNameAddr for GetNameAddrImpl {
    async fn get_from_packet(&self, packet: &Ipv4Packet) -> String {
        let dest = packet.get_destination();
        let dest_sock_addr = SocketAddr::from(SocketAddrV4::new(dest, 80));
        let result = task::spawn_blocking(move || {
          let name_info: Result<(String, String), dns_lookup::LookupError> = getnameinfo(&dest_sock_addr, 0);
          return name_info.unwrap().0;
        }).await;

        return result.unwrap();
    }
}
