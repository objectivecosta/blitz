use arp::spoofer::ArpSpoofer;
use operating_system::network_tools::NetworkTools;
use packet_inspection::inspector::Inspector;

pub mod packet_inspection;
pub mod arp;
pub mod private;
pub mod operating_system;

// TODO: (@objectivecosta) Make sure to spoof for all clients in the network when Firewall is ready.

#[tokio::main]
async fn main() {
    // We will spoof ARP packets saying we're the router to the client
    let sending_spoofer = arp::spoofer::ArpSpooferImpl::new(private::GATEWAY_IP);
    sending_spoofer.startForIp(private::TARGET_IP);

    let tools = operating_system::network_tools::NetworkToolsImpl::new();
    tools.debug_iterate();

    let hw_addr = tools.fetch_hardware_address("en0");

    let ipv4 = tools.fetch_ipv4_address("en0");
    let ipv6 = tools.fetch_ipv6_address("en0");
    println!("HW address for en0: {}", hw_addr);
    println!("IPv4 address for en0: {}", ipv4);
    println!("IPv6 address for en0: {}", ipv6.unwrap_or("not available".to_string()));

    // TODO: (@objectivecosta) Implement this part.
    // We will spoof ARP packets saying we're the client to the router 
    // let receiving_spoofer = arp::spoofer::ArpSpooferImpl::new();
}