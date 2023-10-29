use async_trait::async_trait;
use pnet::datalink::Channel::{self};
use pnet::datalink::{self, NetworkInterface, DataLinkSender, DataLinkReceiver};
use pnet::packet::ethernet::{EtherTypes, EthernetPacket, MutableEthernetPacket};
use pnet::packet::ipv4::Ipv4Packet;
use pnet::packet::Packet;
use pnet::util::MacAddr;
use tokio::sync::watch;
use tokio::{task, join};
use std::sync::Arc;
use std::thread::JoinHandle;
use std::time::SystemTime;

use crate::logger::sqlite_logger::Logger;
use crate::private::SELF_IP_OBJ;
use crate::socket::socket_manager::{self, SocketManager};
use crate::socket::socket_reader::SocketReader;

use super::get_name_addr::{GetNameAddr, GetNameAddrImpl};

#[async_trait]
pub trait Inspector {
    async fn start_inspecting(&mut self);
}

// pub struct AsyncInspectorImpl {
//     _impl: Arc<std::sync::Mutex<InspectorImpl>>
// }

pub struct InspectorImpl<'a> {
    socket_manager: &'a SocketManager,
    get_name_addr: Arc<tokio::sync::Mutex<dyn GetNameAddr + Send>>,
    logger: Arc<tokio::sync::Mutex<Box<dyn Logger + Send>>>
}


impl<'a> InspectorImpl<'a> {
    pub fn new(socket_manager: &'a SocketManager, logger: Box<dyn Logger + Send>) -> Self {
        let mut result = Self {
            socket_manager: socket_manager,
            get_name_addr: Arc::from(tokio::sync::Mutex::new(GetNameAddrImpl::new())),
            logger: Arc::from(tokio::sync::Mutex::new(logger))
        };

        return result;
    }
}


#[async_trait]
impl Inspector for InspectorImpl<'_> {
    async fn start_inspecting(&mut self) {
        loop {
            let packet_vector = self.socket_manager.recv().await;
            self.process_ethernet_packet(packet_vector.to_packet());
        }
    }
}

impl InspectorImpl<'_> {
    fn process_ethernet_packet(&self, packet: EthernetPacket) {
        let src = packet.get_source().to_string();
        let tgt = packet.get_destination().to_string();

        match packet.get_ethertype() {
            EtherTypes::Ipv4 => {
                self.process_ipv4_packet(packet.packet());
            }
            EtherTypes::Ipv6 => {
                println!("Received new IPv6 packet; Ethernet properties => src='{}';target='{}'", src, tgt);
            }
            EtherTypes::Arp => {
                // println!("Received new Arp packet; Ethernet properties => src='{}';target='{}'", src, tgt);
            }
            default => {
                let packet_type = packet.get_ethertype().to_string();
                println!("Received new Ethernet packet src='{}';target='{}';type='{}'", src, tgt, packet_type);
            }
        }
    }

    fn process_ipv4_packet(&self, packet: &[u8]) {
        let ethernet_packet = EthernetPacket::new(packet.clone()).unwrap();
        let ipv4_packet = Ipv4Packet::new(ethernet_packet.payload()).unwrap();

        println!("Processing IPv4 packet! src='{}';target='{}';", ipv4_packet.get_source().to_string(), ipv4_packet.get_destination().to_string());

        let moved_packet = ipv4_packet.packet().to_owned();

        let get_name_addr = self.get_name_addr.clone();
        let logger = self.logger.clone();

        let now = SystemTime::now();
        let timestamp: i64 = now.duration_since(SystemTime::UNIX_EPOCH).unwrap().as_secs().try_into().unwrap();

        // Spawn logger process... this can take as much time as possible since it's async.
        tokio::spawn(async move {
            let get_name_addr_lock = get_name_addr.lock();
            let mut get_name_addr = get_name_addr_lock.await;

            let logger_lock = logger.lock();
            let logger = logger_lock.await;

            let packet: Ipv4Packet<'_> = Ipv4Packet::new(&moved_packet).unwrap();

            let source = packet.get_source();
            let destination = packet.get_destination();

            let source_dns = get_name_addr.get_from_packet(&source).await;
            let destination_dns = get_name_addr.get_from_packet(&destination).await;

            logger.log_traffic(
                timestamp,
                source.to_string().as_str(),
                source_dns.as_str(),
                destination.to_string().as_str(),
                destination_dns.as_str(),
                moved_packet.len() as i64,
                packet.payload().len() as i64,
            );
        });
    }
}
