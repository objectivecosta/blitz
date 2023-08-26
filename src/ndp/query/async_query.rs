use std::{
    collections::HashMap,
    net::Ipv6Addr,
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

use crate::ndp::network_location::V6NetworkLocation;

use super::{/*query_sender::NdpQuerySenderImpl, */query_listener::NdpQueryListenerImpl};

#[async_trait]
pub trait AsyncNdpQueryExecutor {
    async fn query_multiple(&self, all_ips: Vec<Ipv6Addr>) -> HashMap<Ipv6Addr, MacAddr>;
    async fn query(&self, ipv4: Ipv6Addr) -> MacAddr;
}

pub struct AsyncNdpQueryExecutorImpl {
    // _sender: Arc<Mutex<NdpQuerySenderImpl>>,
    _listener: Arc<Mutex<NdpQueryListenerImpl>>,
    cancellation_token: Arc<AtomicBool>,
}

impl AsyncNdpQueryExecutorImpl {
    pub fn new(interface: NetworkInterface, location: V6NetworkLocation) -> Self {
        let cancellation_token = Arc::from(AtomicBool::new(false));
        Self {
            /*_sender: Arc::from(Mutex::new(NdpQuerySenderImpl::new(
                interface.clone(),
                location,
                cancellation_token.clone(),
            ))),*/
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

        // // Sender
        // let sender = self._sender.clone();
        // let sender_future = task::spawn_blocking(move || {
        //     let lock = sender.lock();
        //     let mut sender = lock.unwrap();

        //     let result = sender.query_multiple(all_ips);
        //     return result;
        // });

        let timeout = timeout(Duration::from_millis(1000), listener_future);
        let value = tokio::spawn(timeout).await;

        // This makes sure we release the lock!
        self.cancellation_token
            .store(true, std::sync::atomic::Ordering::Relaxed);

        // This fetches the result!
        let result = result_map.lock().unwrap();
        return result.clone();
    }

    async fn query(&self, ipv4: Ipv6Addr) -> MacAddr {
        return MacAddr::zero();
        // let executor = self._sender.clone();
        // let cancellation_token = self.cancellation_token.clone();
        // let future = task::spawn_blocking(move || {
        //     let lock = executor.lock();
        //     let mut executor = lock.unwrap();
        //     cancellation_token.store(false, std::sync::atomic::Ordering::Relaxed);
        //     let result = executor.query(ipv4);
        //     return result;
        // });

        // let timeout = timeout(Duration::from_millis(1000), future);

        // let join_handle = tokio::spawn(timeout);

        // let value = join_handle.await;

        // let result = value.unwrap();

        // if let Ok(mac_addr) = result {
        //     return mac_addr.unwrap();
        // } else {
        //     self.cancellation_token
        //         .store(true, std::sync::atomic::Ordering::Relaxed);
        //     return MacAddr::zero();
        // }
    }
}