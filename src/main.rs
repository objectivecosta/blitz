use arp::spoofer::ArpSpoofer;
use operating_system::network_tools::NetworkTools;
use packet_inspection::inspector::Inspector;

use crate::arp::spoofer::Mitm;

pub mod packet_inspection;
pub mod arp;
pub mod private;
pub mod operating_system;

// TODO: (@objectivecosta) Make sure to spoof for all clients in the network when Firewall is ready.

#[tokio::main]
async fn main() {
    // We will spoof ARP packets saying we're the router to the client
    let tools = operating_system::network_tools::NetworkToolsImpl::new();

    let interface = tools.fetch_interface("en0");
    let hw_addr = tools.fetch_hardware_address("en0").unwrap();
    let ipv4 = tools.fetch_ipv4_address("en0").unwrap();

    let mitm = Mitm {
        hw: hw_addr,
        ipv4
    };

    let gateway = std::net::Ipv4Addr::from([***REMOVED***]);
    
    let sending_spoofer = arp::spoofer::ArpSpooferImpl::new(
        interface,
        mitm,
        gateway
    );

    let target = std::net::Ipv4Addr::from([***REMOVED***]);
    sending_spoofer.spoof_target(target);
}