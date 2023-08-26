// use std::{
//     collections::HashMap,
//     net::{Ipv6Addr, Ipv6Addr},
//     sync::{atomic::AtomicBool, Arc, Mutex},
// };

// use futures::channel::mpsc;
// use pnet::{
//     datalink::{self, Channel, DataLinkSender, NetworkInterface},
//     packet::{
//         ndp::NdpPacket,
//         ethernet::{EtherTypes, EthernetPacket},
//         Packet,
//     },
//     util::MacAddr,
// };

// use crate::ndp::{
//     network_location::V6NetworkLocation,
//     packet_builder::{NdpPacketBuilder, NdpPacketBuilderImpl},
// };

// pub struct NdpQuerySenderImpl {
//     interface: NetworkInterface,
//     current_location: V6NetworkLocation,
//     sender: Option<Arc<Mutex<Box<dyn DataLinkSender>>>>,
//     cancellation_token: Arc<AtomicBool>,
// }

// impl NdpQuerySenderImpl {
//     pub fn new(
//         interface: NetworkInterface,
//         location: V6NetworkLocation,
//         cancellation_token: Arc<AtomicBool>,
//     ) -> Self {
//         let (_, mut _abort_signal) = mpsc::channel::<u8>(16);

//         let mut res = Self {
//             interface,
//             current_location: location,
//             sender: None,
//             cancellation_token: cancellation_token,
//         };

//         res.setup_socket();

//         return res;
//     }

//     pub fn query(&mut self, ipv4: Ipv6Addr) -> MacAddr {
//         let mut result: HashMap<Ipv6Addr, MacAddr> = HashMap::new();
//         self.query_multiple(vec![ipv4]);
//         return result[&ipv4];
//     }

//     fn setup_socket(&mut self) {
//         let (tx, rx) = match datalink::channel(&self.interface, Default::default()) {
//             Ok(Channel::Ethernet(tx, rx)) => (tx, rx),
//             Ok(_) => panic!("Unhandled channel type"),
//             Err(e) => panic!(
//                 "An error occurred when creating the datalink channel: {}",
//                 e
//             ),
//         };

//         self.sender = Some(Arc::from(Mutex::new(tx)));
//     }

//     pub fn query_multiple(&mut self, all_ips: Vec<Ipv6Addr>) {
//         let slice = all_ips.as_slice();
//         let query_packets: Vec<Vec<u8>> = slice
//             .into_iter()
//             .map(|ipv4| {
//                 return self.make_query_packet(*ipv4);
//             })
//             .collect();

//         let mut sender = self.sender.clone();

//         if let Some(sender) = sender.as_mut() {
//             let mut sender = sender.lock().unwrap();

//             for query_packet in query_packets {
//                 let ethernet_packet = EthernetPacket::new(&query_packet).unwrap();
//                 let ndp_packet = NdpPacket::new(ethernet_packet.payload()).unwrap();
//                 println!(
//                     "Sending request for {}",
//                     ndp_packet.get_target_proto_addr().to_string()
//                 );

//                 _ = sender.send_to(query_packet.as_slice(), None);
//             }
//         }
//     }

//     fn make_query_packet(&self, ipv6: Ipv6Addr) -> Vec<u8> {
//         let builder = NdpPacketBuilderImpl::new();
//         let target = V6NetworkLocation {
//             ipv6: ipv4,
//             hw: MacAddr::broadcast(),
//         };

//         let ndp_request = builder.build_request(self.current_location, target);
//         let ethernet_request = builder.wrap_in_ethernet(
//             self.current_location.hw,
//             target.hw,
//             EtherTypes::Ndp,
//             ndp_request,
//         );

//         return ethernet_request;
//     }
// }
