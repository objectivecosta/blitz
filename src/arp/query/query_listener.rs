use std::{
    collections::HashMap,
    net::Ipv4Addr,
    sync::{atomic::AtomicBool, Arc, Mutex},
};

use pnet::{
    datalink::{self, Channel, DataLinkReceiver, NetworkInterface},
    packet::{
        arp::ArpPacket,
        ethernet::{EtherTypes, EthernetPacket},
        Packet,
    },
    util::MacAddr,
};
use tokio::sync::mpsc;

use crate::arp::network_location::NetworkLocation;

pub struct ArpQueryListenerImpl {
    interface: NetworkInterface,
    current_location: NetworkLocation,
    receiver: Option<Arc<Mutex<Box<dyn DataLinkReceiver>>>>,
    cancellation_token: Arc<AtomicBool>,
}

impl ArpQueryListenerImpl {
    pub fn new(
        interface: NetworkInterface,
        location: NetworkLocation,
        cancellation_token: Arc<AtomicBool>,
    ) -> Self {
        let (_, mut _abort_signal) = mpsc::channel::<u8>(16);

        let mut res = Self {
            interface,
            current_location: location,
            receiver: None,
            cancellation_token: cancellation_token,
        };

        res.setup_socket();

        return res;
    }

    pub fn listen_for(&mut self, addresses: &[Ipv4Addr], result: &mut HashMap<Ipv4Addr, MacAddr>) {
        let size = addresses.len();
        // let mut result: HashMap<Ipv4Addr, MacAddr> = HashMap::new();
        let mut receiver = self.receiver.clone();
        let receiver = receiver.as_mut().unwrap();
        let mut receiver = receiver.lock().unwrap();

        loop {
            if self
                .cancellation_token
                .load(std::sync::atomic::Ordering::Relaxed)
                == true
            {
                return;
            } else {
                match receiver.next() {
                    Ok(packet) => {
                        let packet = EthernetPacket::new(packet).unwrap();
                        let res = self.process_query_response(packet, &addresses);

                        if let Ok((ipv4, mac_addr)) = res {
                            result.insert(ipv4, mac_addr);
                        }

                        if result.len() == size {
                            break;
                        }
                    }
                    Err(e) => {
                        // If an error occurs, we can handle it here
                        panic!("An error occurred while reading: {}", e);
                    }
                }
            }
        }
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

        self.receiver = Some(Arc::from(Mutex::new(rx)));
    }

    fn process_query_response(
        &self,
        packet: EthernetPacket,
        all_ips: &[Ipv4Addr],
    ) -> Result<(Ipv4Addr, MacAddr), ()> {
        if packet.get_ethertype() != EtherTypes::Arp {
            return Err(());
        }

        let arp_packet = ArpPacket::new(packet.payload());

        if arp_packet.is_none() {
            return Err(());
        }

        let arp_packet = arp_packet.unwrap();

        let sender_hw_address = arp_packet.get_sender_hw_addr();
        let sender_proto_address = arp_packet.get_sender_proto_addr();

        println!(
            "Got response for {} ({})",
            sender_proto_address.to_string(),
            sender_hw_address.to_string()
        );

        if all_ips.contains(&sender_proto_address) {
            return Ok((sender_proto_address, sender_hw_address));
        } else {
            return Err(());
        }
    }
}
