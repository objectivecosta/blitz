use arp::spoofer::ArpSpoofer;
use packet_inspection::inspector::Inspector;

pub mod packet_inspection;
pub mod arp;
pub mod private;

// TODO: (@objectivecosta) Make sure to spoof for all clients in the network when Firewall is ready.

#[tokio::main]
async fn main() {
    // We will spoof ARP packets saying we're the router to the client
    let sending_spoofer = arp::spoofer::ArpSpooferImpl::new(private::GATEWAY_IP);
    sending_spoofer.startForIp(private::TARGET_IP);

    // TODO: (@objectivecosta) Implement this part.
    // We will spoof ARP packets saying we're the client to the router 
    // let receiving_spoofer = arp::spoofer::ArpSpooferImpl::new();
}