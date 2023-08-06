use pnet::{packet::{ethernet::{MutableEthernetPacket, EtherType}, arp::{ArpOperation, MutableArpPacket, ArpHardwareType}, MutablePacket}, util::MacAddr};

use super::network_location::NetworkLocation;


pub trait ArpPacketBuilder {
  fn build_request(&self, sender: NetworkLocation, target: NetworkLocation) -> Vec<u8>;
  fn build_response(&self, sender: NetworkLocation, target: NetworkLocation) -> Vec<u8>;
  fn wrap_in_ethernet(&self, sender: MacAddr, target: MacAddr, ether_type: EtherType, packet: Vec<u8>) -> Vec<u8>;
}

pub struct ArpPacketBuilderImpl;

impl ArpPacketBuilderImpl {
  pub fn new() -> Self{
    Self {

    }
  }

  fn build(&self, is_request: bool, sender: NetworkLocation, target: NetworkLocation) -> Vec<u8> {
    let operation = if is_request { ArpOperation::new(1) } else { ArpOperation::new(2) };

    let mut arp_packet_buffer = [0u8; 28];
    let mut arp_packet = MutableArpPacket::new(&mut arp_packet_buffer).unwrap();
    arp_packet.set_hardware_type(ArpHardwareType::new(1)); // Ethernet
    arp_packet.set_protocol_type(EtherType::new(0x0800)); // IPv4
    arp_packet.set_hw_addr_len(6); // ethernet is 6 long
    arp_packet.set_proto_addr_len(4); // ipv4s is 4 long
    arp_packet.set_operation(operation); // 1 is request; 2 is reply.
    arp_packet.set_sender_hw_addr(sender.hw);
    arp_packet.set_sender_proto_addr(sender.ipv4);
    arp_packet.set_target_hw_addr(target.hw); // 0xff on all is Broadcast
    arp_packet.set_target_proto_addr(target.ipv4);

    return arp_packet.packet_mut().to_vec();
  }
}

impl ArpPacketBuilder for ArpPacketBuilderImpl {
  fn build_request(&self, sender: NetworkLocation, target: NetworkLocation) -> Vec<u8> {
    return self.build(true, sender, target);
  }

  fn build_response(&self, sender: NetworkLocation, target: NetworkLocation) -> Vec<u8> {
    return self.build(false, sender, target);
  }

  fn wrap_in_ethernet(&self, sender: MacAddr, target: MacAddr, ether_type: EtherType, packet: Vec<u8>) -> Vec<u8> {
    let mut ethernet_buffer = [0u8; 42];
        let mut ethernet_packet = MutableEthernetPacket::new(&mut ethernet_buffer).unwrap();
        ethernet_packet.set_destination(target);
        ethernet_packet.set_source(sender);
        ethernet_packet.set_ethertype(ether_type);
        ethernet_packet.set_payload(packet.as_slice());

      return ethernet_packet.packet_mut().to_vec();
  }
}