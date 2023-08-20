use std::{
    collections::HashMap,
    net::Ipv4Addr,
    sync::{atomic::AtomicBool, Arc, Mutex},
};

use futures::channel::mpsc;
use pnet::{
    datalink::{self, Channel, DataLinkSender, NetworkInterface},
    packet::{
        arp::ArpPacket,
        ethernet::{EtherTypes, EthernetPacket},
        Packet,
    },
    util::MacAddr,
};

use crate::arp::{
    network_location::NetworkLocation,
    packet_builder::{ArpPacketBuilder, ArpPacketBuilderImpl},
};

pub struct ArpQuerySenderImpl {
    interface: NetworkInterface,
    current_location: NetworkLocation,
    sender: Option<Arc<Mutex<Box<dyn DataLinkSender>>>>,
    cancellation_token: Arc<AtomicBool>,
}

impl ArpQuerySenderImpl {
    pub fn new(
        interface: NetworkInterface,
        location: NetworkLocation,
        cancellation_token: Arc<AtomicBool>,
    ) -> Self {
        let (_, mut _abort_signal) = mpsc::channel::<u8>(16);

        let mut res = Self {
            interface,
            current_location: location,
            sender: None,
            cancellation_token: cancellation_token,
        };

        res.setup_socket();

        return res;
    }

    pub fn query(&mut self, ipv4: Ipv4Addr) -> MacAddr {
        let mut result: HashMap<Ipv4Addr, MacAddr> = HashMap::new();
        self.query_multiple(vec![ipv4]);
        return result[&ipv4];
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

        self.sender = Some(Arc::from(Mutex::new(tx)));
    }

    pub fn query_multiple(&mut self, all_ips: Vec<Ipv4Addr>) {
        let slice = all_ips.as_slice();
        let query_packets: Vec<Vec<u8>> = slice
            .into_iter()
            .map(|ipv4| {
                return self.make_query_packet(*ipv4);
            })
            .collect();

        let mut sender = self.sender.clone();

        if let Some(sender) = sender.as_mut() {
            let mut sender = sender.lock().unwrap();

            for query_packet in query_packets {
                let ethernet_packet = EthernetPacket::new(&query_packet).unwrap();
                let arp_packet = ArpPacket::new(ethernet_packet.payload()).unwrap();
                println!(
                    "Sending request for {}",
                    arp_packet.get_target_proto_addr().to_string()
                );

                _ = sender.send_to(query_packet.as_slice(), None);
            }
        }
    }

    fn make_query_packet(&self, ipv4: Ipv4Addr) -> Vec<u8> {
        let builder = ArpPacketBuilderImpl::new();
        let target = NetworkLocation {
            ipv4: ipv4,
            hw: MacAddr::broadcast(),
        };

        let arp_request = builder.build_request(self.current_location, target);
        let ethernet_request = builder.wrap_in_ethernet(
            self.current_location.hw,
            target.hw,
            EtherTypes::Arp,
            arp_request,
        );

        return ethernet_request;
    }
}
