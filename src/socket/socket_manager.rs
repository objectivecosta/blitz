use std::sync::Arc;

use pnet::{
    datalink::{self, Channel, DataLinkReceiver, DataLinkSender, NetworkInterface},
    packet::ethernet::{EthernetPacket, MutableEthernetPacket},
};
use tokio::sync::{watch, Mutex};

use super::{async_socket_manager::AsyncSocketManagerImpl, ethernet_packet_wrapper::EthernetPacketWrapper};

pub trait SocketManager {
    fn acquire_read(&self) -> watch::Receiver<EthernetPacketWrapper>;
}

pub struct SocketManagerImpl {
    _channel_rx: watch::Receiver<EthernetPacketWrapper>,
    _impl: Arc<Mutex<AsyncSocketManagerImpl>>
}

impl SocketManagerImpl {
    pub fn new(network_interface: &NetworkInterface) -> Self {
        let empty: Vec<u8> = vec![];
        let (channel_tx, channel_rx) = watch::channel(EthernetPacketWrapper::new(&empty));
        let _impl = AsyncSocketManagerImpl::new(network_interface, channel_tx);
        let socket_manager: SocketManagerImpl = SocketManagerImpl {
            _channel_rx: channel_rx,
            _impl: Arc::from(Mutex::new(_impl))
        };

        return socket_manager;
    }
}

impl SocketManager for SocketManagerImpl {
    fn acquire_read(&self) -> watch::Receiver<EthernetPacketWrapper> {
        return self._channel_rx.clone();
    }
}

impl SocketManagerImpl {
    pub fn start(&mut self) {
      let clone = self._impl.clone();
      tokio::spawn(async move {
        let mut acquired = clone.lock().await;
        acquired.start();
      });
    }
}
