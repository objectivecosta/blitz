use std::{net::Ipv4Addr, sync::{Arc, Mutex}};

use pnet::{
    datalink::{self, Channel, DataLinkReceiver, DataLinkSender, NetworkInterface},
    packet::{
        arp::{ArpHardwareType, ArpOperation, MutableArpPacket, ArpPacket, Arp},
        ethernet::{EtherType, EtherTypes, EthernetPacket, MutableEthernetPacket},
        MutablePacket, Packet,
    },
    util::MacAddr,
};

use super::spoofer::NetworkLocation;

pub trait ArpQueryExecutor {
    fn query(&mut self, ipv4: Ipv4Addr) -> MacAddr;
}

pub struct ArpQueryExecutorImpl {
    interface: NetworkInterface,
    current_location: NetworkLocation,
    sender: Option<Arc<Mutex<Box<dyn DataLinkSender>>>>,
    receiver: Option<Arc<Mutex<Box<dyn DataLinkReceiver>>>>,
}

impl ArpQueryExecutorImpl {
    pub fn new(interface: NetworkInterface, location: NetworkLocation) -> Self {
        let mut res = Self {
            interface,
            current_location: location,
            sender: None,
            receiver: None,
        };

        res.setup_socket();

        return res;
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
        self.receiver = Some(Arc::from(Mutex::new(rx)));
    }

    fn make_query_packet(&self, ipv4: Ipv4Addr) -> Vec<u8> {
        let mut arp_packet_buffer = [0u8; 28];
        let mut arp_packet = MutableArpPacket::new(&mut arp_packet_buffer).unwrap();
        arp_packet.set_hardware_type(ArpHardwareType::new(1)); // Ethernet
        arp_packet.set_protocol_type(EtherType::new(0x0800)); // IPv4
        arp_packet.set_hw_addr_len(6); // ethernet is 6 long
        arp_packet.set_proto_addr_len(4); // ipv4s is 4 long
        arp_packet.set_operation(ArpOperation::new(1)); // 1 is request; 2 is reply.
        arp_packet.set_sender_hw_addr(self.current_location.hw);
        arp_packet.set_sender_proto_addr(self.current_location.ipv4);
        arp_packet.set_target_hw_addr(MacAddr(0xff, 0xff, 0xff, 0xff, 0xff, 0xff)); // Broadcast
        arp_packet.set_target_proto_addr(ipv4);

        let mut ethernet_buffer = [0u8; 42];
        let mut ethernet_packet = MutableEthernetPacket::new(&mut ethernet_buffer).unwrap();

        ethernet_packet.set_destination(MacAddr::broadcast());
        ethernet_packet.set_source(self.interface.mac.unwrap());
        ethernet_packet.set_ethertype(EtherTypes::Arp);
        ethernet_packet.set_payload(arp_packet.packet_mut());


        return ethernet_packet.packet().to_owned();
    }

    fn process_query_response(&self, packet: EthernetPacket, searching_for: Ipv4Addr) -> Result<MacAddr, ()> {
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

      println!("process_query_response -> {} = {}", sender_hw_address.to_string(), sender_proto_address.to_string());

      if sender_proto_address == searching_for {
        return Ok(sender_hw_address);
      } else {
        return Err(());
      }
    }
}

impl ArpQueryExecutor for ArpQueryExecutorImpl {
    fn query(&mut self, ipv4: Ipv4Addr) -> MacAddr {
        let query_packet = self.make_query_packet(ipv4);

        let mut sender = self.sender.clone();

        if let Some(sender) = sender.as_mut() {
            let mut sender = sender.lock().unwrap();
            let send_opt = sender.send_to(query_packet.as_slice(), None);

            if let Some(send_res) = send_opt {
                if let Ok(_) = send_res {
                    println!("Sent packet successfully!");
                } else {
                    println!("Failed on second part!");
                }
            } else {
                println!("Failed on first part!");
            }
        }

        let mut receiver = self.receiver.clone();
        let receiver = receiver.as_mut().unwrap();
        let mut receiver = receiver.lock().unwrap();

        loop {
          match receiver.next() {
              Ok(packet) => {
                  let packet = EthernetPacket::new(packet).unwrap();
                  let res = self.process_query_response(packet, ipv4);

                  if let Ok(mac_addr) = res {
                    return mac_addr;
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
