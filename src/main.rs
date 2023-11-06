
use std::sync::Arc;

use clap::Parser;
use logger::sqlite_logger::Logger;
use operating_system::network_tools::NetworkTools;

use crate::{operating_system::network_tools::NetworkToolsImpl, logger::sqlite_logger::SQLiteLogger, socket::socket_manager::SocketManager, packet_inspection::inspector::InspectorImpl};

pub mod logger;
pub mod operating_system;
pub mod packet_inspection;
pub mod socket;

/// Search for a pattern in a file and display the lines that contain it.
#[derive(Parser)]
struct BlitzParameters {
    #[arg(short)]
    input_interface: String,
    #[arg(short)]
    output_interface: String,
}

#[tokio::main(flavor = "multi_thread", worker_threads = 4)]
async fn main() {
    let network_tools = NetworkToolsImpl::new();
    let parameters = BlitzParameters::parse();
    let input_interface_name = parameters.input_interface.as_str();
    let output_interface_name = parameters.output_interface.as_str();
    
    let input_interface = network_tools.fetch_interface(input_interface_name);
    let input_hw_address = network_tools.fetch_hardware_address(input_interface_name).unwrap();
    
    let output_interface = network_tools.fetch_interface(output_interface_name);
    let output_hw_address = network_tools.fetch_hardware_address(output_interface_name).unwrap();

    let path = format!("./db.sqlite");
    let logger = SQLiteLogger::new(path.as_str());

    logger.setup_table();

    let input_channel = tokio::sync::mpsc::channel::<Arc<[u8]>>(64);
    let input_manager: SocketManager = SocketManager::new(&input_interface, input_channel.0);

    let output_channel = tokio::sync::mpsc::channel::<Arc<[u8]>>(64);
    let output_manager: SocketManager = SocketManager::new(&output_interface, output_channel.0);

    let logger: Box<dyn Logger + Send> = Box::from(logger);
    let shared_logger = Arc::from(tokio::sync::Mutex::new(logger));

    let input_inspector = InspectorImpl::new("inbound", shared_logger.clone(), output_hw_address, input_hw_address);
    let output_inspector = InspectorImpl::new("outbound", shared_logger.clone(), input_hw_address, output_hw_address);

    let mut input_to_output_receiver = input_channel.1;
    let mut output_to_input_receiver = output_channel.1;

    let input_to_output = tokio::task::spawn(async move {
        loop {
            let packet = input_to_output_receiver.recv().await.unwrap();

            if input_inspector.process_ethernet_packet(packet.clone()) {
                output_manager.send(packet).await;
            }
        }
    });

    let output_to_input = tokio::task::spawn(async move {
        loop {
            let packet = output_to_input_receiver.recv().await.unwrap();
            if output_inspector.process_ethernet_packet(packet.clone()) {
                input_manager.send(packet).await;
            }
        }
    });

    let _ = tokio::join!(input_to_output, output_to_input);
}
