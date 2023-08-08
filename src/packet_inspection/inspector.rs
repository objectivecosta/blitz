use async_trait::async_trait;
use pnet::datalink::Channel::{self};
use pnet::datalink::{self, NetworkInterface, DataLinkSender, DataLinkReceiver};
use pnet::packet::ethernet::{EtherTypes, EthernetPacket, MutableEthernetPacket};
use pnet::packet::ipv4::Ipv4Packet;
use pnet::packet::Packet;
use pnet::util::MacAddr;
use tokio::{task, join};
use std::sync::Arc;
use std::thread::JoinHandle;

use crate::arp::network_location::NetworkLocation;
use crate::logger::sqlite_logger::Logger;
use crate::private::SELF_IP_OBJ;

use super::get_name_addr::{GetNameAddr, GetNameAddrImpl};

#[async_trait]
pub trait AsyncInspector {
    async fn start_inspecting(&self);
}

// pub struct EthernetChannel {
//     tx: Box<dyn DataLinkSender>,
//     rx: Box<dyn DataLinkReceiver>,
// }

// impl EthernetChannel {
//     pub fn new(tx: Box<dyn DataLinkSender>,
//         rx: Box<dyn DataLinkReceiver>) -> Self {
//             Self {
//                 tx, rx
//             }
//         }
// }

pub struct AsyncInspectorImpl {
    _impl: Arc<std::sync::Mutex<InspectorImpl>>
}

pub struct InspectorImpl {
    interface: NetworkInterface,
    gateway_location: NetworkLocation,
    target_location: NetworkLocation, // TODO(objectivecosta): Make this a proper ARP cache instead of just a single target.
    get_name_addr: Arc<tokio::sync::Mutex<dyn GetNameAddr + Send>>,
    logger: Arc<tokio::sync::Mutex<Box<dyn Logger + Send>>>,
    sender: Arc<std::sync::Mutex<Box<dyn DataLinkSender>>>,
    receiver: Arc<std::sync::Mutex<Box<dyn DataLinkReceiver>>>,
}

impl AsyncInspectorImpl {
    pub fn new(interface: &NetworkInterface, gateway_location: NetworkLocation, target_location: NetworkLocation, logger: Box<dyn Logger + Send>) -> Self {
        Self {
            _impl: Arc::from(std::sync::Mutex::new(InspectorImpl::new(interface, gateway_location, target_location, logger)))
        }
    }
}

impl InspectorImpl {
    pub fn new(interface: &NetworkInterface, gateway_location: NetworkLocation, target_location: NetworkLocation, logger: Box<dyn Logger + Send>) -> Self {
        let (tx, rx) = Self::setup_socket(&interface);

        let mut result = Self {
            interface: interface.to_owned(),
            gateway_location: gateway_location,
            target_location: target_location,
            get_name_addr: Arc::from(tokio::sync::Mutex::new(GetNameAddrImpl::new())),
            logger: Arc::from(tokio::sync::Mutex::new(logger)),
            sender: Arc::from(std::sync::Mutex::new(tx)),
            receiver: Arc::from(std::sync::Mutex::new(rx)),
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

#[async_trait]
impl AsyncInspector for AsyncInspectorImpl {
    async fn start_inspecting(&self) {
        let executor = self._impl.clone();
        let _ = task::spawn_blocking(move || {
            let executor_lock = executor.lock();
            let mut executor = executor_lock.unwrap();
            executor.start_inspecting();
        }).await;
    }
}

impl InspectorImpl {
    fn start_inspecting(&mut self) {
        let receiver = self.receiver.clone();
        let mut receiver = receiver.lock().unwrap();
        loop {
            match receiver.next() {
                Ok(packet) => {
                    let packet = EthernetPacket::new(packet).unwrap();
                    self.process_ethernet_packet(packet);
                }
                Err(e) => {
                    // If an error occurs, we can handle it here
                    panic!("An error occurred while reading: {}", e);
                }
            }
        }
    }
    
    fn process_ethernet_packet(&self, packet: EthernetPacket) {
        let src = packet.get_source().to_string();
        let tgt = packet.get_destination().to_string();

        let is_outgoing = packet.get_source() == self.target_location.hw;
        let is_incoming = packet.get_source() == self.gateway_location.hw;

        match packet.get_ethertype() {
            EtherTypes::Ipv4 => {
                if is_incoming || is_outgoing {
                    self.process_ipv4_packet(packet.packet());
                }
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

    fn forward_outgoing_packet(&self, packet: EthernetPacket) {
        let mut modified_packet = MutableEthernetPacket::owned(packet.packet().to_vec()).unwrap();
        modified_packet.set_destination(self.gateway_location.hw);
        modified_packet.set_source(self.interface.mac.unwrap());

        let sender = self.sender.clone();
        let mut sender = sender.lock().unwrap();

        sender.send_to(modified_packet.packet(), None);
    }

    fn forward_incoming_packet(&self, packet: EthernetPacket) {
        let mut modified_packet = MutableEthernetPacket::owned(packet.packet().to_vec()).unwrap();
        modified_packet.set_destination(self.target_location.hw);
        modified_packet.set_source(self.interface.mac.unwrap());

        let sender = self.sender.clone();
        let mut sender = sender.lock().unwrap();

        sender.send_to(modified_packet.packet(), None);
    }

    fn process_ipv4_packet(&self, packet: &[u8]) {
        let ethernet_packet = EthernetPacket::new(packet.clone()).unwrap();
        let ipv4_packet = Ipv4Packet::new(ethernet_packet.payload()).unwrap();

        let is_incoming_eth = ethernet_packet.get_source() == self.gateway_location.hw;
        let is_incoming_to_target = is_incoming_eth && ipv4_packet.get_destination() == self.target_location.ipv4;
        let is_outgoing = ethernet_packet.get_source() == self.target_location.hw;

        if !is_incoming_to_target && !is_outgoing {
            println!("Skipping IPv4 packet! src='{}' ({});target='{}' ({})", ethernet_packet.get_source().to_string(), ipv4_packet.get_source().to_string(), ethernet_packet.get_destination().to_string(), ipv4_packet.get_destination().to_string());
            return;
        }

        println!("Processing IPv4 packet! src='{}';target='{}';is_incoming_to_target={};is_outgoing={}", ipv4_packet.get_source().to_string(), ipv4_packet.get_destination().to_string(), is_incoming_to_target, is_outgoing);

        if is_outgoing {
            self.forward_outgoing_packet(EthernetPacket::new(packet.clone()).unwrap())
        }

        let moved_packet = ipv4_packet.packet().to_owned();

        let get_name_addr = self.get_name_addr.clone();
        let logger = self.logger.clone();

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
