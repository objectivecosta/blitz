use async_trait::async_trait;
use pnet::datalink::Channel::{self};
use pnet::datalink::{self, NetworkInterface, DataLinkSender, DataLinkReceiver};
use pnet::packet::ethernet::{EtherTypes, EthernetPacket};
use pnet::packet::ipv4::Ipv4Packet;
use pnet::packet::Packet;
use tokio::task;
use std::sync::{Arc, Mutex};

use crate::logger::sqlite_logger::Logger;
use crate::private::SELF_IP_OBJ;

use super::get_name_addr::{GetNameAddr, GetNameAddrImpl};

#[async_trait]
pub trait AsyncInspector {
    async fn start_inspecting(&self);
}

pub struct AsyncInspectorImpl {
    _impl: Arc<Mutex<InspectorImpl>>
}

pub struct InspectorImpl {
    interface: NetworkInterface,
    get_name_addr: Arc<tokio::sync::Mutex<dyn GetNameAddr + Send>>,
    logger: Arc<tokio::sync::Mutex<dyn Logger + Send>>,

    sender: Option<Arc<Mutex<Box<dyn DataLinkSender>>>>,
    receiver: Option<Arc<Mutex<Box<dyn DataLinkReceiver>>>>,
}

impl AsyncInspectorImpl {
    pub fn new(interface: &NetworkInterface, logger: Arc<tokio::sync::Mutex<dyn Logger + Send>>) -> Self {
        Self {
            _impl: Arc::from(Mutex::new(InspectorImpl::new(interface, logger)))
        }
    }
}

impl InspectorImpl {
    pub fn new(interface: &NetworkInterface, logger: Arc<tokio::sync::Mutex<dyn Logger + Send>>) -> Self {
        let mut result = Self {
            interface: interface.to_owned(),
            get_name_addr: Arc::from(tokio::sync::Mutex::from(GetNameAddrImpl::new())),
            logger: logger,
            sender: None,
            receiver: None
        };

        result.setup_socket();

        return result;
    }

    fn setup_socket(&mut self) {
        let (tx, rx) = match datalink::channel(&self.interface, Default::default()) {
            Ok(Channel::Ethernet(tx, rx)) => (tx, rx),
            Ok(_) => panic!("Unhandled channel type"),
            Err(e) => panic!("An error occurred when creating the datalink channel: {}", e)
        };

        self.sender = Some(Arc::from(Mutex::new(tx)));
        self.receiver = Some(Arc::from(Mutex::new(rx)));
    }
}

#[async_trait]
impl AsyncInspector for AsyncInspectorImpl {
    async fn start_inspecting(&self) {
        let executor = self._impl.clone();
        task::spawn_blocking(move || {
            let executor_lock = executor.lock();
            let mut executor = executor_lock.unwrap();
            executor.start_inspecting();
        });
    }
}

impl InspectorImpl {
    fn start_inspecting(&mut self) {
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
    
    fn process_ethernet_packet(&self, packet: EthernetPacket) {
        let src = packet.get_source().to_string();
        let tgt = packet.get_destination().to_string();

        match packet.get_ethertype() {
            EtherTypes::Ipv4 => {
                println!("Received new IPv4 packet; Ethernet properties => src='{}';target='{}'", src, tgt);
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
                println!("Received new Ethernet packet src='{}';target='{}';type='{}'", src, tgt, packet_type);
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
