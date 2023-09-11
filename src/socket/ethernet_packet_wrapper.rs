use pnet::packet::ethernet::EthernetPacket;

pub struct EthernetPacketWrapper {
  data: Vec<u8>,
}

impl EthernetPacketWrapper {
  pub fn new(packet: &[u8]) -> Self {
    return EthernetPacketWrapper {
      data: packet.to_vec()
    };
  }
  pub fn to_packet(&self) -> EthernetPacket {
      return EthernetPacket::new(&self.data.as_slice()).unwrap();
  }
}