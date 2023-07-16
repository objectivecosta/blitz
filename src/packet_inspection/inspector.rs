use async_trait::async_trait;
use pnet::datalink::{self, NetworkInterface, DataLinkReceiver};
use pnet::datalink::Channel::Ethernet;
use pnet::packet::{Packet, MutablePacket};
use pnet::packet::ethernet::{EthernetPacket, MutableEthernetPacket};
use tokio::sync::Mutex;

use std::env;
use std::sync::Arc;

#[async_trait]
pub trait Inspector {
  async fn start_inspecting(&mut self);
}

pub struct InspectorImpl {
    interface: Box<NetworkInterface>,
    // receiver: Arc<Mutex<Box<dyn DataLinkReceiver>>>,
}

impl InspectorImpl {
  pub fn new(
    interface: Box<NetworkInterface>,
    // receiver: Box<dyn DataLinkReceiver>
) -> Self {
    Self {
        interface: interface //,
        // receiver: Arc::from(Mutex::new(receiver))
    }
  }
}

#[async_trait]
impl Inspector for InspectorImpl {
  async fn start_inspecting(&mut self) {
    // Create a new channel, dealing with layer 2 packets
    let (_, mut rx) = match datalink::channel(&self.interface, Default::default()) {
        Ok(Ethernet(tx, rx)) => (tx, rx),
        Ok(_) => panic!("Unhandled channel type"),
        Err(e) => panic!("An error occurred when creating the datalink channel: {}", e)
    };

    loop {
        match rx.next() {
            Ok(packet) => {
                let packet = EthernetPacket::new(packet).unwrap();
                println!("Received new Ethernet packet of size: {}", packet.packet().len())
            },
            Err(e) => {
                // If an error occurs, we can handle it here
                panic!("An error occurred while reading: {}", e);
            }
        }
    }
  }
}