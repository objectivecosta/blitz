use async_trait::async_trait;
use pnet::datalink::{self, NetworkInterface};
use pnet::datalink::Channel::Ethernet;
use pnet::packet::ipv4::Ipv4Packet;
use pnet::packet::{Packet, MutablePacket};
use pnet::packet::ethernet::{EthernetPacket, MutableEthernetPacket, EtherType, EtherTypes};

use std::{env, default};

use crate::packet_inspection::get_name_addr;

use super::get_name_addr::{GetNameAddr, GetNameAddrImpl};

#[async_trait]
pub trait Inspector {
  async fn start_inspecting(&self);
}

pub struct InspectorImpl {
    interface: NetworkInterface
}

impl InspectorImpl {
  pub fn new(interface: &NetworkInterface) -> Self {
    Self {
        interface: interface.to_owned()
    }
  }
}

#[async_trait]
impl Inspector for InspectorImpl {
  async fn start_inspecting(&self) {
    let interface_names_match =
        |iface: &NetworkInterface| iface == &self.interface;

    // Find the network interface with the provided name
    let interfaces = datalink::interfaces();
    let interface = interfaces.into_iter()
                              .filter(interface_names_match)
                              .next()
                              .unwrap();

    // Create a new channel, dealing with layer 2 packets
    let (mut tx, mut rx) = match datalink::channel(&interface, Default::default()) {
        Ok(Ethernet(tx, rx)) => (tx, rx),
        Ok(_) => panic!("Unhandled channel type"),
        Err(e) => panic!("An error occurred when creating the datalink channel: {}", e)
    };

    loop {
        match rx.next() {
            Ok(packet) => {
                let packet = EthernetPacket::new(packet).unwrap();
                self.process_ethernet_packet(packet);
            },
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
        },
        EtherTypes::Ipv6 => {
          println!("Received new IPv6 packet; Ethernet properties => src='{}';target='{}'", src, tgt);
        },
        EtherTypes::Arp => {
          println!("Received new Arp packet; Ethernet properties => src='{}';target='{}'", src, tgt);
        },
        default => {
          let packet_type = packet.get_ethertype().to_string();
          println!("Received new Ethernet packet src='{}';target='{}';type='{}'", src, tgt, packet_type);
        }
    }
  }

  fn process_ipv4_packet(&self, packet: &[u8]) {
    let ipv4_packet = Ipv4Packet::new(packet).unwrap();
    println!("Processing IPv4 packet! src='{}';target='{}';type='{}'", ipv4_packet.get_source().to_string(), ipv4_packet.get_destination().to_string(), "IPv4");

    let moved_packet = packet.to_owned();
    
    let get_name_addr = GetNameAddrImpl{};
    tokio::spawn(async move {
      let packet: Ipv4Packet<'_> = Ipv4Packet::new(&moved_packet).unwrap();
      println!("GetNameAddr spawned for IP: {};", &packet.get_destination().to_string());
      let res = get_name_addr.get_from_packet(&packet).await;
      println!("GetNameAddr res for IP: {} = {}", &packet.get_destination().to_string(), res);
    });
  }
}