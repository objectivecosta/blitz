use std::{
    sync::Arc,
    time::{SystemTime, UNIX_EPOCH},
};

use chrono::{DateTime, Utc};
use operating_system::network_tools::NetworkTools;

use pnet::util::MacAddr;
use tokio::task;

use crate::{
    arp::{
        network_location::NetworkLocation,
        query::{AsyncArpQueryExecutor, AsyncArpQueryExecutorImpl},
        spoofer::{AsyncArpSpoofer, SpoofingEntry},
    },
    logger::sqlite_logger::SQLiteLogger,
    packet_inspection::{
        inspector::{AsyncInspector, AsyncInspectorImpl},
        inspector_vintage::{InspectorVintage, InspectorVintageImpl},
    },
};

pub mod arp;
pub mod logger;
pub mod operating_system;
pub mod packet_inspection;
pub mod private;

// TODO: (@objectivecosta) Make sure to spoof for all clients in the network when Firewall is ready.

#[tokio::main(flavor = "multi_thread", worker_threads = 10)]
async fn main() {
    // We will spoof ARP packets saying we're the router to the client
    let tools = operating_system::network_tools::NetworkToolsImpl::new();
    let gateway_ip_fetched = tools.fetch_gateway_ip();

    let en0_interface = tools.fetch_interface("en0");
    let en0_hw_addr = tools.fetch_hardware_address("en0").unwrap();
    let en0_ipv4 = tools.fetch_ipv4_address("en0").unwrap();

    let now = SystemTime::now();
    let date_time: DateTime<Utc> = chrono::DateTime::from(now);
    let date_time_format = "%Y-%m-%d-%H-%M-%S";
    let formatted = date_time.format(date_time_format).to_string();

    let path = format!("./db-{}.sqlite", formatted);
    let logger = SQLiteLogger::new(path.as_str());

    logger.migrate();

    let inspector_location = NetworkLocation {
        hw: en0_hw_addr,
        ipv4: en0_ipv4,
    };

    let gateway_ip = std::net::Ipv4Addr::from([***REMOVED***]);
    let target = std::net::Ipv4Addr::from([***REMOVED***]);

    let query = AsyncArpQueryExecutorImpl::new(en0_interface.clone(), inspector_location);
    let target_hw_addr = query.query(target).await;

    let gateway_mac_addr = query.query(gateway_ip).await;

    let gateway_mac_addr_cached = query.query(gateway_ip_fetched).await;

    let gateway_location = NetworkLocation {
        ipv4: gateway_ip,
        hw: gateway_mac_addr,
    };

    println!(
        "Target MacAddr: {}; Gateway Fetched: {}; Gateway Fixed: {}",
        target_hw_addr.to_string(),
        gateway_mac_addr,
        gateway_mac_addr_cached
    );

    // let inspector_vintage = InspectorVintageImpl::new(
    //     &en0_interface,
    //     gateway_mac_addr_cached,
    //     target_hw_addr,
    //     Arc::from(tokio::sync::Mutex::from(logger))
    // );
    // let inspector = inspector_vintage;

    let inspector = AsyncInspectorImpl::new(
        &en0_interface,
        gateway_location,
        NetworkLocation {
            ipv4: target,
            hw: target_hw_addr,
        },
        Box::new(logger),
    );

    let target_location = NetworkLocation {
        ipv4: target,
        hw: target_hw_addr,
    };


    let mut spoofer = arp::spoofer::AsyncArpSpooferImpl::new(en0_interface);

    spoofer.add_entry(SpoofingEntry::new(gateway_location, inspector_location.hw, target_location)).await;

    tokio::join!(/*inspector.start_inspecting(),*/ spoofer.start_spoofing());
}
