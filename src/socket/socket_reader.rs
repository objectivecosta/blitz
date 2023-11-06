use std::sync::{Arc, Mutex};

use pnet::datalink::DataLinkReceiver;
use tokio::sync::{watch, mpsc};

use super::ethernet_packet_vector::EthernetPacketVector;

pub struct SocketReader {
    rx: Arc<Mutex<Box<dyn DataLinkReceiver>>>,
    sender: mpsc::Sender<Arc<[u8]>>,
    // receiver: mpsc::Receiver<EthernetPacketVector>,
}

impl SocketReader {
    pub fn new(rx: Box<dyn DataLinkReceiver>, sender: mpsc::Sender<Arc<[u8]>>) -> Self {
        let reader: SocketReader = SocketReader {
            rx: Arc::from(Mutex::new(rx)),
            sender,
        };

        return reader;
    }

    pub fn start(&self) {
        let rx = self.rx.clone();
        let sender = self.sender.clone();
        tokio::task::spawn_blocking(move || {
            let mut locked_rx = rx.lock().unwrap();
            // let locked_sender = sender.lock().unwrap();

            loop {
                match locked_rx.next() {
                    Ok(packet) => {
                        let _ = sender.blocking_send(packet.into());
                    }
                    Err(e) => {
                        // If an error occurs, we can handle it here
                        panic!("An error occurred while reading: {}", e);
                    }
                }
            }
        });
    }
}
