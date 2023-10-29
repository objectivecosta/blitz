use pnet::datalink::NetworkInterface;

use super::{datalink_provider::DataLinkProvider, socket_reader::SocketReader, socket_writer::SocketWriter, ethernet_packet_vector::EthernetPacketVector};

pub struct SocketManager {
    reader: SocketReader,
    writer: SocketWriter,
}

impl SocketManager {
    pub fn new(network_interface: &NetworkInterface) -> Self {
        let (ethernet_tx, ethernet_rx) = (DataLinkProvider {}).provide(network_interface);
        let socket_manager = SocketManager {
            reader: SocketReader::new(ethernet_rx),
            writer: SocketWriter::new(ethernet_tx)
        };

        return socket_manager;
    }

    pub async fn recv(&self) -> EthernetPacketVector {
        return self.reader.recv().await;
    }

    pub async fn send(&self, packet: EthernetPacketVector) -> bool {
        return self.writer.send(packet).await;
    }
}
