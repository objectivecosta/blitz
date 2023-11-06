use std::sync::{Arc, Mutex};

use pnet::{datalink::DataLinkSender, packet::ethernet::{EthernetPacket, MutableEthernetPacket}};


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

    pub async fn send(&self, packet: Arc<[u8]>) -> bool {
        let packet = packet.clone();
        let size = packet.len();
        let tx = self.tx.clone();
        let task = tokio::task::spawn_blocking(move || {
            let mut locked = tx.lock().unwrap();
            let result = locked.send_to(packet.as_ref(), None).unwrap();
            return result;
        });

        let final_result = match task.await {
            Ok(_) => true,
            Err(_) => false,
        };

        println!("Sending packet of size: {}", size);

        return final_result;
    }
}
