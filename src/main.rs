use std::{sync::Arc, time::{SystemTime, UNIX_EPOCH}};

use arp::{spoofer::ArpSpoofer, query::{ArpQueryExecutorImpl, ArpQueryExecutor}};
use operating_system::network_tools::NetworkTools;
use packet_inspection::inspector::{Inspector, self, InspectorImpl};
use pnet::util::MacAddr;
use tokio::sync::Mutex;

use crate::{arp::spoofer::{NetworkLocation}, logger::sqlite_logger::SQLiteLogger};

pub mod packet_inspection;
pub mod arp;
pub mod private;
pub mod operating_system;
pub mod logger;

// TODO: (@objectivecosta) Make sure to spoof for all clients in the network when Firewall is ready.

#[tokio::main]
async fn main() {
    // We will spoof ARP packets saying we're the router to the client
    let tools = operating_system::network_tools::NetworkToolsImpl::new();

    let en0_interface = tools.fetch_interface("en0");
    let en0_hw_addr = tools.fetch_hardware_address("en0").unwrap();
    let en0_ipv4 = tools.fetch_ipv4_address("en0").unwrap();

    let now = SystemTime::now();
    let since_the_epoch = now.duration_since(UNIX_EPOCH).expect("Time exists");

    let path = format!("./db-{}.sqlite", since_the_epoch.as_millis());
    let logger = SQLiteLogger::new(path.as_str());

    logger.migrate();

    let inspector = InspectorImpl::new(
        &en0_interface, 
        Arc::from(Mutex::from(logger))
    );

    let inspector_location = NetworkLocation {
        hw: en0_hw_addr,
        ipv4: en0_ipv4
    };

    let gateway_ip = std::net::Ipv4Addr::from([***REMOVED***]);
    let target = std::net::Ipv4Addr::from([***REMOVED***]);

    let mut query = ArpQueryExecutorImpl::new(en0_interface, inspector_location);

    let target_mac_addr = query.query(target);

    println!("Target MacAddr: {}", target_mac_addr.to_string());
    
    // let mut sending_spoofer = arp::spoofer::ArpSpooferImpl::new(
    //     en0_interface,
    //     inspector_location,
    //     gateway
    // );

    // let target = std::net::Ipv4Addr::from([***REMOVED***]);
    // let target_hw_addr = MacAddr::new(***REMOVED***);
    // sending_spoofer.spoof_target(target);

    // tokio::join!(inspector.start_inspecting());
}