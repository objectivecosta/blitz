use pnet::datalink::{self, Channel, DataLinkReceiver, DataLinkSender, NetworkInterface};
use tokio::sync::watch::{self, Sender};

use super::{datalink_provider::DataLinkProvider, socket_reader::SocketReader, socket_writer::SocketWriter};

pub(super) struct SocketManager {
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
}
