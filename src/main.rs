use std::{sync::{Arc, atomic::AtomicBool}, time::{SystemTime, UNIX_EPOCH}};

use arp::{spoofer::ArpSpoofer, query::{ArpQueryExecutorImpl}};
use operating_system::network_tools::NetworkTools;
use packet_inspection::inspector::{Inspector, self, InspectorImpl};
use pnet::util::MacAddr;
use tokio::sync::Mutex;

use crate::{arp::{spoofer::{NetworkLocation}, query::{AsyncArpQueryExecutorImpl, AsyncArpQueryExecutor}}, logger::sqlite_logger::SQLiteLogger};

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
    let gateway_ip_fetched = tools.fetch_gateway_ip();

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
    let target = std::net::Ipv4Addr::from([***REMOVED*** + 20]);

    let query = AsyncArpQueryExecutorImpl::new(en0_interface.clone(), inspector_location);
    let target_mac_addr = query.query(target).await;

    let gateway_mac_addr = query.query(gateway_ip).await;

    let gateway_mac_addr_cached = query.query(gateway_ip_fetched).await;

    println!("Target MacAddr: {}; Gateway Fetched: {}; Gateway Fixed: {}", target_mac_addr.to_string(), gateway_mac_addr, gateway_mac_addr_cached);
    
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