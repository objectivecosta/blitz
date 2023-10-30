use std::sync::{Arc, Mutex};

use pnet::datalink::DataLinkSender;

use super::ethernet_packet_vector::EthernetPacketVector;

pub struct SocketWriter {
    tx: Arc<Mutex<Box<dyn DataLinkSender>>>,
}

impl SocketWriter {
    pub fn new(tx: Box<dyn DataLinkSender>) -> Self {
        let writer = SocketWriter {
            tx: Arc::from(Mutex::new(tx)),
        };

        return writer;
    }

    pub async fn send(&self, packet: EthernetPacketVector) -> bool {
        let tx = self.tx.clone();
        let result = tokio::task::spawn_blocking(move || {
            let mut locked = tx.lock().unwrap();
            return locked.send_to(packet.to_slice(), None);
        })
        .await;

        match result {
            Ok(_) => return true,
            Err(_) => return false,
        }
    }
}
