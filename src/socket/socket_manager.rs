use std::sync::Arc;

use pnet::datalink::NetworkInterface;
use tokio::sync::{watch, mpsc};

use super::{
    datalink_provider::DataLinkProvider,
    socket_reader::SocketReader, socket_writer::SocketWriter,
};

pub struct SocketManager {
    reader: SocketReader,
    writer: SocketWriter,
}

impl SocketManager {
    pub fn new(
        network_interface: &NetworkInterface,
        sender: mpsc::Sender<Arc<[u8]>>,
    ) -> Self {
        let (ethernet_tx, ethernet_rx) = (DataLinkProvider::new()).provide(network_interface);
        let socket_manager = SocketManager {
            reader: SocketReader::new(ethernet_rx, sender),
            writer: SocketWriter::new(ethernet_tx),
        };

        socket_manager.reader.start();

        return socket_manager;
    }

    pub async fn send(&self, packet: Arc<[u8]>) -> bool {
        return self.writer.send(packet).await;
    }
}
