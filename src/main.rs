
use clap::Parser;
use operating_system::network_tools::NetworkTools;

use crate::{operating_system::network_tools::NetworkToolsImpl, logger::sqlite_logger::SQLiteLogger, socket::socket_manager::SocketManager, packet_inspection::inspector::{InspectorImpl, Inspector}, forwarder::forwarder::Forwarder};

pub mod logger;
pub mod operating_system;
pub mod packet_inspection;
pub mod private;
pub mod socket;
pub mod forwarder;

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
    let network_tools = NetworkToolsImpl::new();
    let parameters = BlitzParameters::parse();
    let input_interface_name = parameters.input_interface.as_str();
    let output_interface_name = parameters.input_interface.as_str();
    
    let input_interface = network_tools.fetch_interface(input_interface_name);
    let input_hw_address = network_tools.fetch_hardware_address(input_interface_name).unwrap();
    
    let output_interface = network_tools.fetch_interface(output_interface_name);
    let output_hw_address = network_tools.fetch_hardware_address(output_interface_name).unwrap();

    let path = format!("./db.sqlite");
    let logger = SQLiteLogger::new(path.as_str());

    logger.setup_table();

    let input_manager: SocketManager = SocketManager::new(&input_interface);
    let output_manager: SocketManager = SocketManager::new(&output_interface);

    let mut inspector = InspectorImpl::new(&input_manager, Box::new(logger));
    let inspector_future = inspector.start_inspecting();

    let forwarder = Forwarder::new(&input_manager, &output_manager);
    let forwader_future = forwarder.start_forwarding();

    tokio::join!(inspector_future, forwader_future);
}
