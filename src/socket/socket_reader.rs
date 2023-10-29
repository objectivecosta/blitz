use pnet::datalink::DataLinkReceiver;
use tokio::sync::watch;

use super::ethernet_packet_vector::EthernetPacketVector;

pub struct SocketReader {
    rx: Box<dyn DataLinkReceiver>,
    sender: watch::Sender<EthernetPacketVector>,
    receiver: watch::Receiver<EthernetPacketVector>,
}

impl SocketReader {
    pub fn new(rx: Box<dyn DataLinkReceiver>) -> Self {
        let channel = tokio::sync::watch::channel(EthernetPacketVector::new(vec![].as_slice()));
        let mut reader = SocketReader {
            rx,
            sender: channel.0,
            receiver: channel.1,
        };

        reader.start();

        return reader;
    }

    pub async fn recv(&mut self) -> EthernetPacketVector {
        let _ = self.receiver.changed().await;
        return (*self.receiver.borrow()).copy();
    }

    pub fn start(&mut self) {
        tokio::task::spawn_blocking(move || {
            loop {
                match self.rx.next() {
                    Ok(packet) => {
                        println!("Got packet of size: {}", packet.len());
                        let _ = self.sender.send(EthernetPacketVector::new(packet));
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