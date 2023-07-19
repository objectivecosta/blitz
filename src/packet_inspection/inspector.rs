use async_trait::async_trait;
use pnet::datalink::{self, NetworkInterface};
use pnet::datalink::Channel::Ethernet;
use pnet::packet::{Packet, MutablePacket};
use pnet::packet::ethernet::{EthernetPacket, MutableEthernetPacket};

use std::env;

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
                let src = packet.get_source().to_string();
                let tgt = packet.get_destination().to_string();
                let packet_type = packet.get_ethertype().to_string();
                println!("Received new Ethernet packet src='{}';target='{}';type='{}'", src, tgt, packet_type);
            },
            Err(e) => {
                // If an error occurs, we can handle it here
                panic!("An error occurred while reading: {}", e);
            }
        }
    }
  }
}