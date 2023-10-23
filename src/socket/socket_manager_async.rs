use std::sync::Arc;

use pnet::{
    datalink::{self, Channel, DataLinkReceiver, DataLinkSender, NetworkInterface},
    packet::ethernet::{EthernetPacket, MutableEthernetPacket},
};
use tokio::sync::{watch, Mutex};

use super::{socket_manager::SocketManagerImpl, ethernet_packet_wrapper::EthernetPacketWrapper};
pub trait SocketManager {
    fn acquire_read(&self) -> watch::Receiver<EthernetPacketWrapper>;
}

pub struct AsyncSocketManagerImpl {
    _channel_rx: watch::Receiver<EthernetPacketWrapper>,
    _impl: Arc<Mutex<SocketManagerImpl>>
}

// TODO: (objectivecosta) rewrite this using spawn_blocking
impl AsyncSocketManagerImpl {
    pub fn new(network_interface: &NetworkInterface) -> Self {
        let empty: Vec<u8> = vec![];
        let (channel_tx, channel_rx) = watch::channel(EthernetPacketWrapper::new(&empty));
        let _impl = SocketManagerImpl::new(network_interface, channel_tx);
        let socket_manager: AsyncSocketManagerImpl = AsyncSocketManagerImpl {
            _channel_rx: channel_rx,
            _impl: Arc::from(Mutex::new(_impl))
        };

        return socket_manager;
    }
}

impl SocketManager for AsyncSocketManagerImpl {
    fn acquire_read(&self) -> watch::Receiver<EthernetPacketWrapper> {
        return self._channel_rx.clone();
    }
}

impl AsyncSocketManagerImpl {
    pub async fn  start(&mut self) {
      let clone = self._impl.clone();
      tokio::spawn(async move {
        let mut acquired = clone.lock().await;
        acquired.start();
      });
    }
}
