use std::{
    net::Ipv4Addr,
    sync::{atomic::AtomicBool, Arc, Mutex},
    time::Duration, collections::HashMap,
};

use async_trait::async_trait;
use pnet::{
    datalink::{self, Channel, DataLinkReceiver, DataLinkSender, NetworkInterface},
    packet::{
        arp::ArpPacket,
        ethernet::{EtherTypes, EthernetPacket},
        Packet,
    },
    util::MacAddr,
};
use tokio::{
    sync::mpsc::{self},
    task,
    time::timeout,
};

use super::{
    network_location::NetworkLocation,
    packet_builder::{ArpPacketBuilder, ArpPacketBuilderImpl},
};

#[async_trait]
pub trait AsyncArpQueryExecutor {
    async fn query_multiple(&self, all_ips: Vec<Ipv4Addr>) -> HashMap<Ipv4Addr, MacAddr>;
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
            _impl: Arc::from(Mutex::new(ArpQueryExecutorImpl::new(
                interface,
                location,
                cancellation_token.clone(),
            ))),
            cancellation_token: cancellation_token,
        }
    }
}

#[async_trait]
impl AsyncArpQueryExecutor for AsyncArpQueryExecutorImpl {
    async fn query_multiple(&self, all_ips: Vec<Ipv4Addr>) -> HashMap<Ipv4Addr, MacAddr> {
        let executor = self._impl.clone();
        let cancellation_token = self.cancellation_token.clone();
        let result_map: Arc<Mutex<HashMap<Ipv4Addr, MacAddr>>> = Arc::from(Mutex::new(HashMap::new()));

        let result_clone = result_map.clone();
        let future = task::spawn_blocking(move || {
            let lock = executor.lock();
            let mut executor = lock.unwrap();

            let result_lock = result_clone.lock();
            let mut result = result_lock.unwrap();

            cancellation_token.store(false, std::sync::atomic::Ordering::Relaxed);
            let result = executor.query_multiple(all_ips, &mut result);
            return result;
        });

        let timeout = timeout(Duration::from_millis(1000), future);
        let value = tokio::spawn(timeout).await;

        // let value = join_handle.await;

        let result = value.unwrap();

        // This makes sure we release the lock!
        self.cancellation_token.store(true, std::sync::atomic::Ordering::Relaxed);

        // This fetches the result!
        let abc = result_map.as_ref().lock().unwrap();

        if let Ok(_) = result {
            
            return abc.clone();
        } else {

            return abc.clone();
                }
    }

    async fn query(&self, ipv4: Ipv4Addr) -> MacAddr {
        let executor = self._impl.clone();
        let cancellation_token = self.cancellation_token.clone();
        let future = task::spawn_blocking(move || {
            let lock = executor.lock();
            let mut executor = lock.unwrap();
            cancellation_token.store(false, std::sync::atomic::Ordering::Relaxed);
            let result = executor.query(ipv4);
            return result;
        });

        let timeout = timeout(Duration::from_millis(1000), future);

        let join_handle = tokio::spawn(timeout);

        let value = join_handle.await;

        let result = value.unwrap();

        if let Ok(mac_addr) = result {
            return mac_addr.unwrap();
        } else {
            self.cancellation_token
                .store(true, std::sync::atomic::Ordering::Relaxed);
            return MacAddr::zero();
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
    pub fn new(
        interface: NetworkInterface,
        location: NetworkLocation,
        cancellation_token: Arc<AtomicBool>,
    ) -> Self {
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

    fn query_multiple(&mut self, all_ips: Vec<Ipv4Addr>, result: &mut HashMap<Ipv4Addr, MacAddr>){
        let size = all_ips.len();
        let slice = all_ips.as_slice();
        let query_packets: Vec<Vec<u8>> = slice.into_iter().map(|ipv4| {
            return self.make_query_packet(*ipv4);
        }).collect();

        // let mut response_map: HashMap<Ipv4Addr, MacAddr> = HashMap::new();

        let mut sender = self.sender.clone();

        if let Some(sender) = sender.as_mut() {
            let mut sender = sender.lock().unwrap();


            for query_packet in query_packets {
                _ = sender.send_to(query_packet.as_slice(), None);
            }
        }

        let mut receiver = self.receiver.clone();
        let receiver = receiver.as_mut().unwrap();
        let mut receiver = receiver.lock().unwrap();

        loop {
            if self
                .cancellation_token
                .load(std::sync::atomic::Ordering::Relaxed)
                == true
            {
                return;
            } else {
                match receiver.next() {
                    Ok(packet) => {
                        let packet = EthernetPacket::new(packet).unwrap();
                        let res = self.process_query_response(packet, &slice);

                        if let Ok((ipv4, mac_addr)) = res {
                            result.insert(ipv4, mac_addr);
                        }

                        if result.len() == size {
                            break;
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

    fn query(&mut self, ipv4: Ipv4Addr) -> MacAddr {
        let mut result: HashMap<Ipv4Addr, MacAddr> = HashMap::new();
        self.query_multiple(vec![ipv4], &mut result);
        return result[&ipv4];
    }

    fn make_query_packet(&self, ipv4: Ipv4Addr) -> Vec<u8> {
        let builder = ArpPacketBuilderImpl::new();
        let target = NetworkLocation {
            ipv4: ipv4,
            hw: MacAddr::broadcast(),
        };

        let arp_request = builder.build_request(self.current_location, target);
        let ethernet_request = builder.wrap_in_ethernet(
            self.current_location.hw,
            target.hw,
            EtherTypes::Arp,
            arp_request,
        );

        return ethernet_request;
    }

    fn process_query_response(
        &self,
        packet: EthernetPacket,
        all_ips: &[Ipv4Addr]
    ) -> Result<(Ipv4Addr, MacAddr), ()> {
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

        if all_ips.contains(&sender_proto_address) {
            return Ok((sender_proto_address, sender_hw_address));
        } else {
            return Err(())
        }

    }
}
