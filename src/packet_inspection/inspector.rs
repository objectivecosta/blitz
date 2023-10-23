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

use crate::arp::network_location::NetworkLocation;
use crate::logger::sqlite_logger::Logger;
use crate::private::SELF_IP_OBJ;
use crate::socket::ethernet_packet_wrapper::EthernetPacketWrapper;
use crate::socket::socket_manager_async::AsyncSocketManagerImpl;

use super::get_name_addr::{GetNameAddr, GetNameAddrImpl};

#[async_trait]
pub trait AsyncInspector {
    async fn start_inspecting(&self);
}

// pub struct AsyncInspectorImpl {
//     _impl: Arc<std::sync::Mutex<InspectorImpl>>
// }

pub struct InspectorImpl {
    packet_receiver: watch::Receiver<EthernetPacketWrapper>,
    get_name_addr: Arc<tokio::sync::Mutex<dyn GetNameAddr + Send>>,
    logger: Arc<tokio::sync::Mutex<Box<dyn Logger + Send>>>
}

// impl AsyncInspectorImpl {
//     pub fn new(interface: &NetworkInterface, gateway_location: NetworkLocation, target_location: NetworkLocation, logger: Box<dyn Logger + Send>) -> Self {
//         Self {
//             _impl: Arc::from(std::sync::Mutex::new(InspectorImpl::new(interface, gateway_location, target_location, logger)))
//         }
//     }
// }

impl InspectorImpl {
    pub fn new(packet_receiver: watch::Receiver<EthernetPacketWrapper>, logger: Box<dyn Logger + Send>) -> Self {
        let mut result = Self {
            packet_receiver: packet_receiver,
            get_name_addr: Arc::from(tokio::sync::Mutex::new(GetNameAddrImpl::new())),
            logger: Arc::from(tokio::sync::Mutex::new(logger))
        };

        // result.setup_socket();

        return result;
    }

    fn setup_socket(interface: &NetworkInterface) -> (Box<dyn DataLinkSender>, Box<dyn DataLinkReceiver>) {
        let channel = match datalink::channel(&interface, Default::default()) {
            Ok(Channel::Ethernet(tx, rx)) => (tx, rx),
            Ok(_) => panic!("Unhandled channel type"),
            Err(e) => panic!("An error occurred when creating the datalink channel: {}", e)
        };

        return channel;
    }
}

// #[async_trait]
// impl AsyncInspector for AsyncInspectorImpl {
//     async fn start_inspecting(&self) {
//         let executor = self._impl.clone();
//         let _ = task::spawn_blocking(move || {
//             let executor_lock = executor.lock();
//             let mut executor = executor_lock.unwrap();
//             executor.start_inspecting();
//         }).await;
//     }
// }

#[async_trait]
impl AsyncInspector for InspectorImpl {
    async fn start_inspecting(&self) {
        let mut rx = self.packet_receiver.clone();
        while rx.changed().await.is_ok() {
            let value = rx.borrow();
            let packet = value.to_packet();
            self.process_ethernet_packet(packet);
        }
    }
}

impl InspectorImpl {
    fn process_ethernet_packet(&self, packet: EthernetPacket) {
        let src = packet.get_source().to_string();
        let tgt = packet.get_destination().to_string();

        match packet.get_ethertype() {
            EtherTypes::Ipv4 => {
                self.process_ipv4_packet(packet.packet());
            }
            EtherTypes::Ipv6 => {
                // println!("Received new IPv6 packet; Ethernet properties => src='{}';target='{}'", src, tgt);
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
