use std::{sync::{Arc, Mutex}, time::Duration};

use async_trait::async_trait;

use pnet::{packet::{ethernet::{EtherType, MutableEthernetPacket, EtherTypes}, arp::{ArpHardwareType, ArpOperation, MutableArpPacket}, MutablePacket}, util::MacAddr, datalink::{self, NetworkInterface, Channel, DataLinkSender, DataLinkReceiver}};
use tokio::{task, time::timeout};

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

    fn build_arp_spoofed_packet(&self, target: NetworkLocation) -> Vec<u8> {
        let mut arp_packet_buffer = [0u8; 28];
        let mut arp_packet = MutableArpPacket::new(&mut arp_packet_buffer).unwrap();
        arp_packet.set_hardware_type(ArpHardwareType::new(1)); // Ethernet
        arp_packet.set_protocol_type(EtherType::new(0x0800)); // IPv4
        arp_packet.set_hw_addr_len(6);  // ethernet is 6 long
        arp_packet.set_proto_addr_len(4);  // ipv4s is 4 long
        arp_packet.set_operation(ArpOperation::new(2));  // 1 is request; 2 is reply.
        arp_packet.set_sender_hw_addr(self.inspector.hw); 
        arp_packet.set_sender_proto_addr(self.gateway.ipv4); 
        arp_packet.set_target_hw_addr(target.hw); // TODO: find this information manually
        arp_packet.set_target_proto_addr(target.ipv4); 

        let mut ethernet_buffer = [0u8; 42];
        let mut ethernet_packet = MutableEthernetPacket::new(&mut ethernet_buffer).unwrap();

        ethernet_packet.set_destination(MacAddr::broadcast());
        ethernet_packet.set_source(self.interface.mac.unwrap());
        ethernet_packet.set_ethertype(EtherTypes::Arp);
        ethernet_packet.set_payload(arp_packet.packet_mut());

        return ethernet_packet.packet_mut().to_vec();
    }

    fn spoof_target(&mut self, target: NetworkLocation) -> bool {
        // instantiate packet

        // TODO: (@objectivecosta) Modify these values
        let packet = &self.build_arp_spoofed_packet(target);
        println!("Assembled packets!");
        println!("Preparing to send packet!");

        if let Some(sender) = self.sender.as_mut() {
            let send_opt = sender.send_to(packet, None);

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