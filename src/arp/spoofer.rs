use std::net::Ipv4Addr;

use pnet::{packet::{ethernet::{EtherType}, arp::{Arp, ArpHardwareType, ArpOperation}}, util::MacAddr};

pub trait ArpSpoofer {
    fn startForIp(&self, ip: &str);
}

pub struct ArpSpooferImpl {
    spoofed: String
}

impl ArpSpooferImpl {
    pub fn new(spoofed: &str) -> Self {
        Self {
            spoofed: spoofed.to_string()
        }
    }
}

impl ArpSpoofer for ArpSpooferImpl {
    fn startForIp(&self, target: &str) {
        // instantiate packet


        // TODO: (@objectivecosta) Modify these values
        let packet = Arp {
            hardware_type: ArpHardwareType::new(1),
            protocol_type: EtherType::new(8),
            hw_addr_len: 10,
            proto_addr_len: 8,
            operation: ArpOperation::new(8),
            sender_hw_addr: MacAddr(8,8,8,8,8,8),
            sender_proto_addr: Ipv4Addr::new(1, 1, 1, 1),
            target_hw_addr: MacAddr(8,8,8,8,8,8),
            target_proto_addr: Ipv4Addr::new(2, 2, 2, 2),
            payload: vec![]
        };

        // TODO: (@objectivecosta) Send packet
        // no-op
    }
}