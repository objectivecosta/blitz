use async_trait::async_trait;
use pnet::datalink::Channel::{Ethernet, self};
use pnet::datalink::{self, NetworkInterface, DataLinkSender, DataLinkReceiver};
use pnet::packet::ethernet::{EtherType, EtherTypes, EthernetPacket, MutableEthernetPacket};
use pnet::packet::ipv4::Ipv4Packet;
use pnet::packet::{MutablePacket, Packet};
use pnet::util::MacAddr;
use tokio::sync::Mutex;

use std::borrow::BorrowMut;
use std::collections::HashMap;
use std::net::Ipv4Addr;
use std::sync::Arc;
use std::{default, env};

use crate::logger::sqlite_logger::Logger;
use crate::packet_inspection::{get_name_addr, inspector};
use crate::private::SELF_IP_OBJ;

use super::get_name_addr::{GetNameAddr, GetNameAddrImpl};

#[async_trait]
pub trait InspectorVintage {
    async fn start_inspecting(&self);
}

pub struct InspectorVintageImpl {
    interface: NetworkInterface,
    gateway_hw: MacAddr,
    target_hw: MacAddr, // TODO(objectivecosta): Make this a proper ARP cache instead of just a single target.
    get_name_addr: Arc<tokio::sync::Mutex<dyn GetNameAddr + Send>>,
    logger: Arc<tokio::sync::Mutex<dyn Logger + Send>>,

    sender: Option<Arc<std::sync::Mutex<Box<dyn DataLinkSender>>>>,
    receiver: Option<Arc<std::sync::Mutex<Box<dyn DataLinkReceiver>>>>,
}

impl InspectorVintageImpl {
    pub fn new(
        interface: &NetworkInterface,
        gateway_hw: MacAddr,
        target_hw: MacAddr,
        logger: Arc<Mutex<dyn Logger + Send>>,
    ) -> Self {
        let mut result = Self {
            interface: interface.to_owned(),
            gateway_hw: gateway_hw,
            target_hw: target_hw,
            get_name_addr: Arc::from(tokio::sync::Mutex::from(GetNameAddrImpl::new())),
            logger: logger,
            sender: None,
            receiver: None,
        };

        result.setup_socket();

        return result;
    }

    fn setup_socket(&mut self) {
        let (tx, rx) = match datalink::channel(&self.interface, Default::default()) {
            Ok(Channel::Ethernet(tx, rx)) => (tx, rx),
            Ok(_) => panic!("Unhandled channel type"),
            Err(e) => panic!(
                "An error occurred when creating the datalink channel: {}",
                e
            ),
        };

        self.sender = Some(Arc::from(std::sync::Mutex::new(tx)));
        self.receiver = Some(Arc::from(std::sync::Mutex::new(rx)));
    }
}

#[async_trait]
impl InspectorVintage for InspectorVintageImpl {
    async fn start_inspecting(&self) {
        let mut receiver = self.receiver.clone();
        let receiver = receiver.as_mut().unwrap();
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
}

impl InspectorVintageImpl {
    fn process_ethernet_packet(&self, packet: EthernetPacket) {
        let src = packet.get_source().to_string();
        let tgt = packet.get_destination().to_string();

        match packet.get_ethertype() {
            EtherTypes::Ipv4 => {
                // println!("Received new IPv4 packet; Ethernet properties => src='{}';target='{}'", src, tgt);
                self.process_ipv4_packet(packet.payload());
            }
            EtherTypes::Ipv6 => {
                // println!("Received new IPv6 packet; Ethernet properties => src='{}';target='{}'", src, tgt);
            }
            EtherTypes::Arp => {
                // println!("Received new Arp packet; Ethernet properties => src='{}';target='{}'", src, tgt);
            }
            default => {
                let packet_type = packet.get_ethertype().to_string();
                // println!("Received new Ethernet packet src='{}';target='{}';type='{}'", src, tgt, packet_type);
            }
        }
    }

    fn process_ipv4_packet(&self, packet: &[u8]) {
        let ipv4_packet = Ipv4Packet::new(packet).unwrap();
        // println!("Processing IPv4 packet! src='{}';target='{}';type='{}'", ipv4_packet.get_source().to_string(), ipv4_packet.get_destination().to_string(), "IPv4");

        let is_sent = ipv4_packet.get_source() == SELF_IP_OBJ;
        let is_received = ipv4_packet.get_destination() == SELF_IP_OBJ;

        if !is_sent && !is_received {
            return;
        }

        let moved_packet = packet.to_owned();

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
