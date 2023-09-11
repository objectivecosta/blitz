use pnet::datalink::{self, Channel, DataLinkReceiver, DataLinkSender, NetworkInterface};
use tokio::sync::watch::{self, Sender};

use super::ethernet_packet_wrapper::EthernetPacketWrapper;


pub(super) struct AsyncSocketManagerImpl {
    ethernet_tx: Box<dyn DataLinkSender>,
    ethernet_rx: Box<dyn DataLinkReceiver>,
    tx: Sender<EthernetPacketWrapper>,
}

impl AsyncSocketManagerImpl {
    fn setup_socket(
        network_interface: &NetworkInterface,
    ) -> (Box<dyn DataLinkSender>, Box<dyn DataLinkReceiver>) {
        let channel = match datalink::channel(&network_interface, Default::default()) {
            Ok(Channel::Ethernet(tx, rx)) => (tx, rx),
            Ok(_) => panic!("Unhandled channel type"),
            Err(e) => panic!(
                "An error occurred when creating the datalink channel: {}",
                e
            ),
        };

        return channel;
    }

    pub fn new(network_interface: &NetworkInterface, tx: Sender<EthernetPacketWrapper>) -> Self {
        let (ethernet_tx, ethernet_rx) = Self::setup_socket(network_interface);
        let socket_manager = AsyncSocketManagerImpl {
            ethernet_tx: ethernet_tx,
            ethernet_rx: ethernet_rx,
            tx: tx,
        };

        return socket_manager;
    }

    pub fn start(&mut self) {
        loop {
            match self.ethernet_rx.next() {
                Ok(packet) => {
                    println!("Got packet of size: {}", packet.len());
                    let _ = self.tx.send(EthernetPacketWrapper::new(packet));
                }
                Err(e) => {
                    // If an error occurs, we can handle it here
                    panic!("An error occurred while reading: {}", e);
                }
            }
        }
    }
}
