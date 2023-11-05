use pnet::datalink::NetworkInterface;
use tokio::sync::watch;

use super::{
    datalink_provider::DataLinkProvider, ethernet_packet_vector::EthernetPacketVector,
    socket_reader::SocketReader, socket_writer::SocketWriter,
};

pub struct SocketManager {
    reader: SocketReader,
    writer: SocketWriter,
}

impl SocketManager {
    pub fn new(network_interface: &NetworkInterface) -> Self {
        let (ethernet_tx, ethernet_rx) = (DataLinkProvider {}).provide(network_interface);
        let socket_manager = SocketManager {
            reader: SocketReader::new(ethernet_rx),
            writer: SocketWriter::new(ethernet_tx),
        };

        socket_manager.reader.start();

        return socket_manager;
    }

    pub async fn recv(&self) -> EthernetPacketVector {
        return self.reader.recv().await;
    }

    pub fn receiver(&self) -> watch::Receiver<EthernetPacketVector> {
        return self.reader.receiver();
    }

    pub async fn send(&self, packet: &EthernetPacketVector) -> bool {
        return self.writer.send(packet).await;
    }
}
