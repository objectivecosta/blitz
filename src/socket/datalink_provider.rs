use pnet::datalink::{NetworkInterface, DataLinkSender, self, DataLinkReceiver, Channel};


pub struct DataLinkProvider {
    
}

impl DataLinkProvider {
    pub fn provide(&self, network_interface: &NetworkInterface) -> (Box<dyn DataLinkSender>, Box<dyn DataLinkReceiver>) {
        let channel = match datalink::channel(&network_interface, Default::default()) {
            Ok(Channel::Ethernet(tx, rx)) => (tx, rx),
            Ok(_) => panic!("Unhandled channel type"),
            Err(e) => panic!(
                "An error occurred when creating the datalink channel: {}",
                e
            ),
        };

        return channel;
    }
}