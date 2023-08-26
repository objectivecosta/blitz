use std::{
    collections::HashMap,
    net::Ipv6Addr,
    sync::{atomic::AtomicBool, Arc, Mutex},
};

use pnet::{
    datalink::{self, Channel, DataLinkReceiver, NetworkInterface},
    packet::{
        ethernet::{EtherTypes, EthernetPacket},
        Packet, icmpv6::{ndp::{NdpOptionPacket, NeighborAdvert, NeighborAdvertPacket}, Icmpv6Packet, Icmpv6Types}, ipv6::Ipv6Packet, ip::IpNextHeaderProtocols,
    },
    util::MacAddr,
};
use tokio::sync::mpsc;

use crate::ndp::network_location::V6NetworkLocation;

pub struct NdpQueryListenerImpl {
    interface: NetworkInterface,
    current_location: V6NetworkLocation,
    receiver: Option<Arc<Mutex<Box<dyn DataLinkReceiver>>>>,
    cancellation_token: Arc<AtomicBool>,
}

impl NdpQueryListenerImpl {
    pub fn new(
        interface: NetworkInterface,
        location: V6NetworkLocation,
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

    pub fn listen_for(&mut self, addresses: &[Ipv6Addr], result: &mut HashMap<Ipv6Addr, MacAddr>) {
        let size = addresses.len();
        // let mut result: HashMap<Ipv6Addr, MacAddr> = HashMap::new();
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
        all_ips: &[Ipv6Addr],
    ) -> Result<(Ipv6Addr, MacAddr), ()> {
        if packet.get_ethertype() != EtherTypes::Ipv6 {
            return Err(());
        }

        let packet = Ipv6Packet::new(packet.payload()).unwrap(); 
        let is_icmp = packet.get_next_header() == IpNextHeaderProtocols::Icmpv6;

        if !is_icmp {
            return Err(());
        }

        let packet = Icmpv6Packet::new(packet.payload()).unwrap();

        // Types:
        // 133 - Router Solicitation (NDP)
        // 134 - Router Advertisement (NDP)
        // 135 - Neighbor Solicitation (NDP)
        // 136 - Neighbor Advertisement (NDP)

        // TODO: Also work on Router Advert.
        if packet.get_icmpv6_type() != Icmpv6Types::NeighborAdvert {
            return Err(());
        }

        let advert = NeighborAdvertPacket::new(packet.payload()).unwrap();
        let sender_proto_address = advert.get_target_addr().to_string();

        println!(
            "NDP - Got response for ({})",
            // sender_proto_address.to_string(),
            sender_proto_address.to_string()
        );

        return Err(());

        // if all_ips.contains(&sender_proto_address) {
        //     return Ok((sender_proto_address, sender_hw_address));
        // } else {
           
        // }
    }
}
