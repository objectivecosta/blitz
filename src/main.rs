
use chrono::Local;
use clap::Parser;
use operating_system::network_tools::NetworkTools;

use pnet::ipnetwork::Ipv4Network;

use crate::{
    arp::{
        network_location::NetworkLocation,
        spoofer::SpoofingEntry, query::async_query::{AsyncArpQueryExecutorImpl, AsyncArpQueryExecutor},
    },
    logger::sqlite_logger::SQLiteLogger,
    packet_inspection::inspector::{AsyncInspector, InspectorImpl}, private::{GATEWAY_IP_OBJ, TARGET_IP_OBJ}, socket::socket_manager_async::{AsyncSocketManagerImpl, SocketManager},
};

pub mod arp;
pub mod logger;
pub mod operating_system;
pub mod packet_inspection;
pub mod private;
pub mod socket;

/// Search for a pattern in a file and display the lines that contain it.
#[derive(Parser)]
struct BlitzParameters {
    #[arg(short)]
    input_interface: String,
    #[arg(short)]
    output_interface: String,
}

#[tokio::main(flavor = "multi_thread", worker_threads = 10)]
async fn main() {
    // let parameters = BlitzParameters::parse();

    // We will spoof ARP packets saying we're the router to the client
    let tools = operating_system::network_tools::NetworkToolsImpl::new();
    let gateway_ip_fetched = tools.fetch_gateway_ip();

    println!("Gateway detected: {}", gateway_ip_fetched.to_string());

    let en0_interface = tools.fetch_interface("en0");
    let en0_hw_addr = tools.fetch_hardware_address("en0").unwrap();
    let en0_ipv4 = tools.fetch_ipv4_address("en0").unwrap();

    let path = format!("./db.sqlite");
    let logger = SQLiteLogger::new(path.as_str());

    logger.setup_table();

    let mut socket_manager = AsyncSocketManagerImpl::new(&en0_interface);
    socket_manager.start();

    println!("Got here!");

    return;

    let inspector_location = NetworkLocation {
        hw: en0_hw_addr,
        ipv4: en0_ipv4,
    };

    let gateway_ip = GATEWAY_IP_OBJ;
    let target = TARGET_IP_OBJ; 

    // let query = AsyncArpQueryExecutorImpl::new(en0_interface.clone(), inspector_location);
    // let target_hw_addr = query.query(target).await;

    // let gateway_mac_addr = query.query(gateway_ip).await;

    // let gateway_mac_addr_cached = query.query(gateway_ip_fetched).await;

    // let gateway_location = NetworkLocation {
    //     ipv4: gateway_ip,
    //     hw: gateway_mac_addr,
    // };

    // println!(
    //     "Target MacAddr: {}; Gateway Fetched: {}; Gateway Fixed: {}",
    //     target_hw_addr.to_string(),
    //     gateway_mac_addr,
    //     gateway_mac_addr_cached
    // );

    let inspector = InspectorImpl::new(
        socket_manager.acquire_read(),
        Box::new(logger),
    );

    // let target_location = NetworkLocation {
    //     ipv4: target,
    //     hw: target_hw_addr,
    // };

    // let mut spoofer = arp::spoofer::AsyncArpSpooferImpl::new(en0_interface);

    // let spoofing_gateway_to_target = SpoofingEntry::new(gateway_location, inspector_location.hw, target_location);
    // let spoofing_target_to_gateway = SpoofingEntry::new(target_location, inspector_location.hw, gateway_location);

    // spoofer.add_entry(spoofing_gateway_to_target).await;
    // spoofer.add_entry(spoofing_target_to_gateway).await;

    tokio::join!(inspector.start_inspecting());
}

async fn query_all(ipv4_network: &Ipv4Network, query: &AsyncArpQueryExecutorImpl) {
    let all: Vec<std::net::Ipv4Addr> = ipv4_network.into_iter().collect();
    let mut all_spoof_entries: Vec<SpoofingEntry> = vec![];

    let date = Local::now();
    println!("{} - Starting query multiple", date.format("[%Y-%m-%d %H:%M:%S]"));

   let query_multiple = query.query_multiple(all).await;

   for key in query_multiple.keys() {
        let value = query_multiple[key];
        let date = Local::now();
        println!("{} - {}={}", date.format("[%Y-%m-%d %H:%M:%S]"), key.to_string(), value.to_string());
   }

   return;
}
