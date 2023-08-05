use std::{
    net::Ipv4Addr,
    sync::{Arc, Mutex, atomic::AtomicBool},
    time::Duration,
};

use async_trait::async_trait;
use pnet::{
    datalink::{self, Channel, DataLinkReceiver, DataLinkSender, NetworkInterface},
    packet::{
        arp::{Arp, ArpHardwareType, ArpOperation, ArpPacket, MutableArpPacket},
        ethernet::{EtherType, EtherTypes, EthernetPacket, MutableEthernetPacket},
        MutablePacket, Packet,
    },
    util::MacAddr,
};
use tokio::{task, time::timeout, sync::mpsc::{self, Receiver}};

use super::spoofer::NetworkLocation;

#[async_trait]
pub trait AsyncArpQueryExecutor {
    async fn query(&self, ipv4: Ipv4Addr) -> MacAddr;
}

pub struct AsyncArpQueryExecutorImpl {
    _impl: Arc<Mutex<ArpQueryExecutorImpl>>,
    cancellation_token: Arc<AtomicBool>,
}

impl AsyncArpQueryExecutorImpl {
    pub fn new(interface: NetworkInterface, location: NetworkLocation) -> Self {
        let cancellation_token = Arc::from(AtomicBool::new(false));
        Self {
            _impl: Arc::from(Mutex::new(ArpQueryExecutorImpl::new(interface, location, cancellation_token.clone()))),
            cancellation_token: cancellation_token,
        }
    }
}

#[async_trait]
impl AsyncArpQueryExecutor for AsyncArpQueryExecutorImpl {
    async fn query(&self, ipv4: Ipv4Addr) -> MacAddr {
        // let future = async move {
        //   let lock =  executor.lock();
        //   let mut executor = lock.await;

        //   let result = executor.query(ipv4);
        //   return result;
        // };

        let executor = self._impl.clone();
        let cancellation_token = self.cancellation_token.clone();
        let future = task::spawn_blocking(move || {
            let lock = executor.lock();
            let mut executor = lock.unwrap();
            cancellation_token.store(false, std::sync::atomic::Ordering::Relaxed);
            let result = executor.query(ipv4);
            return result;
        });

        let timeout = timeout(Duration::from_millis(3000), future);

        let join_handle = tokio::spawn(timeout);

        let value = join_handle.await;

        let result = value.unwrap();

        if let Ok(mac_addr) = result {
            return mac_addr.unwrap();
        } else {
            self.cancellation_token.store(true, std::sync::atomic::Ordering::Relaxed);
            return MacAddr(0xff, 0xff, 0xff, 0xff, 0xff, 0xff);
        }
    }
}

pub struct ArpQueryExecutorImpl {
    interface: NetworkInterface,
    current_location: NetworkLocation,
    sender: Option<Arc<Mutex<Box<dyn DataLinkSender>>>>,
    receiver: Option<Arc<Mutex<Box<dyn DataLinkReceiver>>>>,
    cancellation_token: Arc<AtomicBool>,
}

impl ArpQueryExecutorImpl {
    pub fn new(interface: NetworkInterface, location: NetworkLocation, cancellation_token: Arc<AtomicBool>) -> Self {
      let (_, mut _abort_signal) = mpsc::channel::<u8>(16);

        let mut res = Self {
            interface,
            current_location: location,
            sender: None,
            receiver: None,
            cancellation_token: cancellation_token,
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

    fn abort_query(&mut self) {
        self.cancellation_token.store(true, std::sync::atomic::Ordering::Relaxed);
    }

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
            if self.cancellation_token.load(std::sync::atomic::Ordering::Relaxed) == true {
              println!("Abort signal!");
              return MacAddr(0x00, 0x00, 0x00, 0x00, 0x00, 0x00);
            } else {
              println!("Receiver signal!");
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

    fn process_query_response(
        &self,
        packet: EthernetPacket,
        searching_for: Ipv4Addr,
    ) -> Result<MacAddr, ()> {
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

        println!(
            "process_query_response -> {} = {}",
            sender_hw_address.to_string(),
            sender_proto_address.to_string()
        );

        if sender_proto_address == searching_for {
            return Ok(sender_hw_address);
        } else {
            return Err(());
        }
    }
}