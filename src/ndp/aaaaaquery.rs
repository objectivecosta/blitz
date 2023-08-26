use std::{
    collections::HashMap,
    net::{Ipv6Addr, Ipv6Addr},
    sync::{atomic::AtomicBool, Arc, Mutex},
    time::Duration,
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

use super::network_location::V6NetworkLocation;

#[async_trait]
pub trait AsyncNdpQueryExecutor {
    async fn query_multiple(&self, all_ips: Vec<Ipv6Addr>) -> HashMap<Ipv6Addr, MacAddr>;
    async fn query(&self, address: Ipv6Addr) -> MacAddr;
}

pub struct AsyncNdpQueryExecutorImpl {
    _sender: Arc<Mutex<NdpQuerySenderImpl>>,
    _listener: Arc<Mutex<NdpQueryListenerImpl>>,
    cancellation_token: Arc<AtomicBool>,
}

impl AsyncNdpQueryExecutorImpl {
    pub fn new(interface: NetworkInterface, location: V6NetworkLocation) -> Self {
        let cancellation_token = Arc::from(AtomicBool::new(false));
        Self {
            _sender: Arc::from(Mutex::new(NdpQuerySenderImpl::new(
                interface.clone(),
                location,
                cancellation_token.clone(),
            ))),
            _listener: Arc::from(Mutex::new(NdpQueryListenerImpl::new(
                interface.clone(),
                location,
                cancellation_token.clone(),
            ))),
            cancellation_token: cancellation_token,
        }
    }
}

#[async_trait]
impl AsyncNdpQueryExecutor for AsyncNdpQueryExecutorImpl {
    async fn query_multiple(&self, all_ips: Vec<Ipv6Addr>) -> HashMap<Ipv6Addr, MacAddr> {
        let result_map: Arc<Mutex<HashMap<Ipv6Addr, MacAddr>>> =
        Arc::from(Mutex::new(HashMap::new()));
        let cancellation_token = self.cancellation_token.clone();

        // Receiver
        let listener = self._listener.clone();
        let addresses = all_ips.clone();
        let result_clone = result_map.clone();
        let listener_future = task::spawn_blocking(move || {
            let lock = listener.lock();
            let mut listener = lock.unwrap();

            let result_lock = result_clone.lock();
            let mut result = result_lock.unwrap();

            cancellation_token.store(false, std::sync::atomic::Ordering::Relaxed);
            listener.listen_for(addresses.as_slice(), &mut result);
        });

        // Sender
        let sender = self._sender.clone();
        let sender_future = task::spawn_blocking(move || {
            let lock = sender.lock();
            let mut sender = lock.unwrap();

            let result = sender.query_multiple(all_ips);
            return result;
        });

        let timeout = timeout(Duration::from_millis(1000), listener_future);
        let value = tokio::spawn(timeout).await;

        // let result = value.unwrap();

        // This makes sure we release the lock!
        self.cancellation_token
            .store(true, std::sync::atomic::Ordering::Relaxed);

        // This fetches the result!
        let result = result_map.lock().unwrap();
        return result.clone();
    }

    async fn query(&self, address: Ipv6Addr) -> MacAddr {
        let executor = self._sender.clone();
        let cancellation_token = self.cancellation_token.clone();
        let future = task::spawn_blocking(move || {
            let lock = executor.lock();
            let mut executor = lock.unwrap();
            cancellation_token.store(false, std::sync::atomic::Ordering::Relaxed);
            let result = executor.query(address);
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

pub struct NdpQuerySenderImpl {
    interface: NetworkInterface,
    current_location: V6NetworkLocation,
    sender: Option<Arc<Mutex<Box<dyn DataLinkSender>>>>,
    cancellation_token: Arc<AtomicBool>,
}

pub struct NdpQueryListenerImpl {
    interface: NetworkInterface,
    current_location: V6NetworkLocation,
    receiver: Option<Arc<Mutex<Box<dyn DataLinkReceiver>>>>,
    cancellation_token: Arc<AtomicBool>,
}

impl NdpQueryListenerImpl {
    pub fn new(
        interface: NetworkInterface,
        location: V6NetworkLocation,
        cancellation_token: Arc<AtomicBool>,
    ) -> Self {
        let (_, mut _abort_signal) = mpsc::channel::<u8>(16);

        let mut res = Self {
            interface,
            current_location: location,
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

        self.receiver = Some(Arc::from(Mutex::new(rx)));
    }

    fn listen_for(&mut self, addresses: &[Ipv6Addr], result: &mut HashMap<Ipv6Addr, MacAddr>) {
        let size = addresses.len();
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
                        let res = self.process_query_response(packet, &addresses);


                        if let Ok((address, mac_addr)) = res {
                            result.insert(address, mac_addr);
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

    fn process_query_response(
        &self,
        packet: EthernetPacket,
        all_ips: &[Ipv6Addr],
    ) -> Result<(Ipv6Addr, MacAddr), ()> {
        if packet.get_ethertype() != EtherTypes::Ndp {
            return Err(());
        }

        let ndp_packet = ArpPacket::new(packet.payload());

        if ndp_packet.is_none() {
            return Err(());
        }

        let ndp_packet = ndp_packet.unwrap();

        let sender_hw_address = ndp_packet.get_sender_hw_addr();
        let sender_proto_address = ndp_packet.get_sender_proto_addr();

        println!("Got response for {} ({})", sender_proto_address.to_string(), sender_hw_address.to_string());

        if all_ips.contains(&sender_proto_address) {
            return Ok((sender_proto_address, sender_hw_address));
        } else {
            return Err(());
        }
    }
}

impl NdpQuerySenderImpl {
    pub fn new(
        interface: NetworkInterface,
        location: V6NetworkLocation,
        cancellation_token: Arc<AtomicBool>,
    ) -> Self {
        let (_, mut _abort_signal) = mpsc::channel::<u8>(16);

        let mut res = Self {
            interface,
            current_location: location,
            sender: None,
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
    }

    fn query_multiple(&mut self, all_ips: Vec<Ipv6Addr>) {
        let slice = all_ips.as_slice();
        let query_packets: Vec<Vec<u8>> = slice
            .into_iter()
            .map(|ipv6| {
                return self.make_query_packet(*ipv6);
            })
            .collect();

        let mut sender = self.sender.clone();

        if let Some(sender) = sender.as_mut() {
            let mut sender = sender.lock().unwrap();

            for query_packet in query_packets {
                let ethernet_packet = EthernetPacket::new(&query_packet).unwrap();
                let arp_packet = ArpPacket::new(ethernet_packet.payload()).unwrap();
                println!("Sending request for {}", arp_packet.get_target_proto_addr().to_string());

                _ = sender.send_to(query_packet.as_slice(), None);
            }
        }
    }

    fn query(&mut self, ipv4: Ipv6Addr) -> MacAddr {
        let mut result: HashMap<Ipv6Addr, MacAddr> = HashMap::new();
        self.query_multiple(vec![ipv4]);
        return result[&ipv4];
    }

    fn make_query_packet(&self, ipv4: Ipv6Addr) -> Vec<u8> {
        let builder = ArpPacketBuilderImpl::new();
        let target = V6NetworkLocation {
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
}
