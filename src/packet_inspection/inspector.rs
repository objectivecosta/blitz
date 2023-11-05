use pnet::packet::ethernet::{EtherTypes, EthernetPacket};
use pnet::packet::ipv4::Ipv4Packet;
use pnet::packet::Packet;
use pnet::packet::ipv6::Ipv6Packet;
use pnet::util::MacAddr;
use std::sync::Arc;
use std::time::SystemTime;

use crate::logger::sqlite_logger::Logger;

use super::get_name_addr::{GetNameAddr, GetNameAddrImpl};

// #[async_trait]
// pub trait Inspector {
//     async fn start_inspecting(&mut self);
// }

// pub struct AsyncInspectorImpl {
//     _impl: Arc<std::sync::Mutex<InspectorImpl>>
// }

pub struct InspectorImpl {
    tag: &'static str, 
    get_name_addr: Arc<tokio::sync::Mutex<dyn GetNameAddr + Send>>,
    logger: Arc<tokio::sync::Mutex<Box<dyn Logger + Send>>>,
    ignore_source_mac_address: MacAddr,
    ignore_target_mac_address: MacAddr,
}

impl InspectorImpl {
    pub fn new(tag: &'static str, logger: Arc<tokio::sync::Mutex<Box<dyn Logger + Send>>>, ignore_source_mac_address: MacAddr, ignore_target_mac_address: MacAddr) -> Self {
        let result: InspectorImpl = Self {
            tag,
            get_name_addr: Arc::from(tokio::sync::Mutex::new(GetNameAddrImpl::new())),
            logger,
            ignore_source_mac_address,
            ignore_target_mac_address,
        };

        return result;
    }
}

impl InspectorImpl {
    pub fn process_ethernet_packet(&self, packet: &EthernetPacket) -> bool {
        let source = packet.get_source();
        let target = packet.get_destination();
        let src = source.to_string();
        let tgt = target.to_string();

        if source == self.ignore_source_mac_address || target == self.ignore_target_mac_address {
            // println!("[{}] Ignoring packet src='{}';target='{}'", self.tag, src, tgt);
            return false;
        }

        match packet.get_ethertype() {
            EtherTypes::Ipv4 => {
                self.process_ipv4_packet(packet.packet());
            }
            EtherTypes::Ipv6 => {
                self.process_ipv6_packet(packet.packet());
            }
            EtherTypes::Arp => {
                let packet_type = packet.get_ethertype().to_string();
                println!(
                    "[{}] Received new Arp packet src='{}';target='{}';type='{}'",
                    self.tag,
                    src, tgt, packet_type
                );
            }
            default => {
                let packet_type = packet.get_ethertype().to_string();
                println!(
                    "[{}] Received new Ethernet ({}) packet src='{}';target='{}';type='{}'",
                    self.tag,
                    default.to_string(),
                    src,
                    tgt,
                    packet_type
                );
            }
        }

        // Allow all packets...
        return true;
    }

    fn process_ipv4_packet(&self, packet: &[u8]) {
        let ethernet_packet = EthernetPacket::new(packet.clone()).unwrap();
        let ipv4_packet = Ipv4Packet::new(ethernet_packet.payload()).unwrap();

        println!(
            "[{}] Processing IPv4 packet! src='{}';target='{}';",
            self.tag,
            ipv4_packet.get_source().to_string(),
            ipv4_packet.get_destination().to_string()
        );

        let moved_packet = ipv4_packet.packet().to_owned();

        let get_name_addr = self.get_name_addr.clone();
        let logger = self.logger.clone();

        let now = SystemTime::now();
        let timestamp: i64 = now
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap()
            .as_secs()
            .try_into()
            .unwrap();

        // Spawn logger process... this can take as much time as possible since it's async.
        tokio::spawn(async move {
            let get_name_addr_lock = get_name_addr.lock();
            let mut get_name_addr = get_name_addr_lock.await;

            let logger_lock = logger.lock();
            let mut logger = logger_lock.await;

            let packet: Ipv4Packet = Ipv4Packet::new(&moved_packet).unwrap();

            let source = packet.get_source();
            let destination = packet.get_destination();

            let source_dns = get_name_addr.get_from_address(&source).await;
            let destination_dns = get_name_addr.get_from_address(&destination).await;

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

    fn process_ipv6_packet(&self, packet: &[u8]) {
        let ethernet_packet = EthernetPacket::new(packet.clone()).unwrap();
        let ipv6_packet = Ipv6Packet::new(ethernet_packet.payload()).unwrap();

        println!(
            "[{}] Processing IPv6 packet! src='{}';target='{}';",
            self.tag,
            ipv6_packet.get_source().to_string(),
            ipv6_packet.get_destination().to_string()
        );

        let moved_packet = ipv6_packet.packet().to_owned();

        let get_name_addr = self.get_name_addr.clone();
        let logger = self.logger.clone();

        let now = SystemTime::now();
        let timestamp: i64 = now
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap()
            .as_secs()
            .try_into()
            .unwrap();

        // Spawn logger process... this can take as much time as possible since it's async.
        tokio::spawn(async move {
            let get_name_addr_lock = get_name_addr.lock();
            let mut get_name_addr = get_name_addr_lock.await;

            let logger_lock = logger.lock();
            let mut logger = logger_lock.await;

            let packet: Ipv6Packet = Ipv6Packet::new(&moved_packet).unwrap();

            let source = packet.get_source();
            let destination = packet.get_destination();

            let source_dns = get_name_addr.get_from_address6(&source).await;
            let destination_dns = get_name_addr.get_from_address6(&destination).await;

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
