use std::sync::{Arc, Mutex};

use pnet::datalink::DataLinkReceiver;
use tokio::sync::watch;

use super::ethernet_packet_vector::EthernetPacketVector;

pub struct SocketReader {
    rx: Arc<Mutex<Box<dyn DataLinkReceiver>>>,
    sender: Arc<Mutex<watch::Sender<EthernetPacketVector>>>,
    receiver: watch::Receiver<EthernetPacketVector>,
}

impl SocketReader {
    pub fn new(rx: Box<dyn DataLinkReceiver>) -> Self {
        let channel = tokio::sync::watch::channel(EthernetPacketVector::new(vec![].as_slice()));
        let mut reader = SocketReader {
            rx: Arc::from(Mutex::new(rx)),
            sender: Arc::from(Mutex::new(channel.0)),
            receiver: channel.1,
        };

        reader.start();

        return reader;
    }

    pub async fn recv(&self) -> EthernetPacketVector {
        let mut clone = self.receiver.clone();
        let _ = clone.changed().await;
        return (*clone.borrow()).copy();
    }

    pub fn start(&mut self) {
        let rx = self.rx.clone();
        let sender = self.sender.clone();
        tokio::task::spawn_blocking(move || {
            let mut locked_rx = rx.lock().unwrap();
            let locked_sender = sender.lock().unwrap();

            loop {
                match locked_rx.next() {
                    Ok(packet) => {
                        println!("Got packet of size: {}", packet.len());
                        let _ = locked_sender.send(EthernetPacketVector::new(packet));
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
