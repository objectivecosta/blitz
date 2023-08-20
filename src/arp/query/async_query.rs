use std::{
    collections::HashMap,
    net::Ipv4Addr,
    sync::{atomic::AtomicBool, Arc, Mutex},
    time::Duration,
};

use async_trait::async_trait;
use pnet::{
    datalink::NetworkInterface,
    util::MacAddr,
};
use tokio::{
    task,
    time::timeout,
};

use crate::arp::network_location::NetworkLocation;

use super::{query_sender::ArpQuerySenderImpl, query_listener::ArpQueryListenerImpl};

#[async_trait]
pub trait AsyncArpQueryExecutor {
    async fn query_multiple(&self, all_ips: Vec<Ipv4Addr>) -> HashMap<Ipv4Addr, MacAddr>;
    async fn query(&self, ipv4: Ipv4Addr) -> MacAddr;
}

pub struct AsyncArpQueryExecutorImpl {
    _sender: Arc<Mutex<ArpQuerySenderImpl>>,
    _listener: Arc<Mutex<ArpQueryListenerImpl>>,
    cancellation_token: Arc<AtomicBool>,
}

impl AsyncArpQueryExecutorImpl {
    pub fn new(interface: NetworkInterface, location: NetworkLocation) -> Self {
        let cancellation_token = Arc::from(AtomicBool::new(false));
        Self {
            _sender: Arc::from(Mutex::new(ArpQuerySenderImpl::new(
                interface.clone(),
                location,
                cancellation_token.clone(),
            ))),
            _listener: Arc::from(Mutex::new(ArpQueryListenerImpl::new(
                interface.clone(),
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
        let result_map: Arc<Mutex<HashMap<Ipv4Addr, MacAddr>>> =
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

        // This makes sure we release the lock!
        self.cancellation_token
            .store(true, std::sync::atomic::Ordering::Relaxed);

        // This fetches the result!
        let result = result_map.lock().unwrap();
        return result.clone();
    }

    async fn query(&self, ipv4: Ipv4Addr) -> MacAddr {
        let executor = self._sender.clone();
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