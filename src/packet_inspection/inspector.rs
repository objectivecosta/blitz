use async_trait::async_trait;
use pnet::datalink::Channel::Ethernet;
use pnet::datalink::{self, NetworkInterface};
use pnet::packet::ethernet::{EtherType, EtherTypes, EthernetPacket, MutableEthernetPacket};
use pnet::packet::ipv4::Ipv4Packet;
use pnet::packet::{MutablePacket, Packet};
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

#[derive(Clone, Copy)]
pub struct InspectorLog {
    bytes_sent: u64,
    bytes_received: u64,
}

#[async_trait]
pub trait Inspector {
    async fn start_inspecting(&self);
}

pub struct InspectorImpl {
    interface: NetworkInterface,
    get_name_addr: Arc<Mutex<dyn GetNameAddr + Send>>,
    cache: Arc<Mutex<HashMap<String, InspectorLog>>>,
    logger: Arc<Mutex<dyn Logger + Send>>
}

impl InspectorImpl {
    pub fn new(interface: &NetworkInterface, logger: Arc<Mutex<dyn Logger + Send>>) -> Self {
        Self {
            interface: interface.to_owned(),
            get_name_addr: Arc::from(Mutex::from(GetNameAddrImpl::new())),
            cache: Arc::from(Mutex::from(HashMap::new())),
            logger: logger,
        }
    }
}

#[async_trait]
impl Inspector for InspectorImpl {
    async fn start_inspecting(&self) {
        let interface_names_match = |iface: &NetworkInterface| iface == &self.interface;

        // Find the network interface with the provided name
        let interfaces = datalink::interfaces();
        let interface = interfaces
            .into_iter()
            .filter(interface_names_match)
            .next()
            .unwrap();

        // Create a new channel, dealing with layer 2 packets
        let (mut tx, mut rx) = match datalink::channel(&interface, Default::default()) {
            Ok(Ethernet(tx, rx)) => (tx, rx),
            Ok(_) => panic!("Unhandled channel type"),
            Err(e) => panic!(
                "An error occurred when creating the datalink channel: {}",
                e
            ),
        };

        loop {
            match rx.next() {
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

impl InspectorImpl {
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
        let cache = self.cache.clone();
        let logger = self.logger.clone();

        tokio::spawn(async move {
            let get_name_addr_lock = get_name_addr.lock();
            let mut get_name_addr = get_name_addr_lock.await;
            let packet: Ipv4Packet<'_> = Ipv4Packet::new(&moved_packet).unwrap();

            let unknown_address = if is_sent && !is_received {
              packet.get_destination()
            } else {
              packet.get_source()
            };

            let res = get_name_addr.get_from_packet(&unknown_address).await;

            let cache_lock = cache.lock();
            let mut cache = cache_lock.await;

            if cache.contains_key(&res) {
                let mut prev = cache.get(&res).unwrap().clone();
                if is_sent {
                  prev.bytes_sent += moved_packet.len() as u64;
                } else if is_received {
                  prev.bytes_received += moved_packet.len() as u64;
                }
                cache.insert(res.clone(), prev);
                println!(
                    "[IPv4] {} => {}. Bytes sent to that: {}kb; Bytes received from that: {}kb",
                    if is_sent { "localhost".to_owned() } else { res.clone() }, 
                    if is_received { "localhost".to_owned() } else { res.clone() },
                    (prev.bytes_sent as f64) / 1024.0,
                    (prev.bytes_received as f64) / 1024.0
                );

                let logger_lock = logger.lock();
                let logger = logger_lock.await;

                logger.log_traffic(to_ip, to_dns, from_ip, from_dns, packet_size, payload_size);
            } else {
                let log = InspectorLog {
                    bytes_received: if is_received {moved_packet.len() as u64 } else { 0 },
                    bytes_sent: if is_sent {moved_packet.len() as u64 } else { 0 },
                };

                cache.insert(res, log);
            }
        });
    }
}
