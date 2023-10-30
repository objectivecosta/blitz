use pnet::packet::ethernet::EthernetPacket;

#[derive(Clone)]
pub struct EthernetPacketVector {
    data: Vec<u8>,
}

impl EthernetPacketVector {
    pub fn new(packet: &[u8]) -> Self {
        return EthernetPacketVector {
            data: packet.to_vec(),
        };
    }

    // TODO: Can be changed to Copy trait?
    pub fn copy(&self) -> EthernetPacketVector {
        EthernetPacketVector {
            data: self.data.clone(),
        }
    }

    pub fn to_packet(&self) -> EthernetPacket {
        return EthernetPacket::new(&self.data.as_slice()).unwrap();
    }

    pub fn to_slice(&self) -> &[u8] {
        return &self.data.as_slice();
    }
}
