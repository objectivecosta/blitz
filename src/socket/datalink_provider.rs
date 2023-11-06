use nix::libc::rand;
use pnet::datalink::{self, Channel, DataLinkReceiver, DataLinkSender, NetworkInterface, Config, ChannelType, FanoutOption, FanoutType};

pub struct DataLinkProvider {}

impl DataLinkProvider {
    pub fn provide(
        &self,
        network_interface: &NetworkInterface,
    ) -> (Box<dyn DataLinkSender>, Box<dyn DataLinkReceiver>) {
        let config: Config = Default::default();

        let fanout = Some(FanoutOption {
            group_id: unsafe { rand() } as u16,
            fanout_type: FanoutType::LB,
            defrag: true,
            rollover: false,
        });

        let config = Config {
            write_buffer_size: 4096 * 2,
            read_buffer_size: 4096 * 2,
            read_timeout: None,
            write_timeout: None,
            channel_type: ChannelType::Layer2,
            bpf_fd_attempts: 1000,
            linux_fanout: fanout, // None,
            promiscuous: true,
        };

        let channel = match datalink::channel(&network_interface, config) {
            Ok(Channel::Ethernet(tx, rx)) => (tx, rx),
            Ok(_) => panic!("Unhandled channel type"),
            Err(e) => panic!(
                "An error occurred when creating the datalink channel: {}",
                e
            ),
        };

        return channel;
    }
}
