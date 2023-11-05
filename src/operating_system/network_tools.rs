use pnet::{datalink::NetworkInterface, util::MacAddr};
use std::{net::Ipv4Addr, process::Command, str::FromStr};

pub trait NetworkTools {
    fn debug_iterate(&self);
    fn fetch_interface(&self, interface_name: &str) -> NetworkInterface;
    fn fetch_hardware_address(&self, interface_name: &str) -> Option<MacAddr>;
    fn fetch_ipv4_address(&self, interface_name: &str) -> Option<std::net::Ipv4Addr>;
    fn fetch_ipv6_address(&self, interface_name: &str) -> Option<String>;
    fn fetch_gateway_ip(&self) -> Ipv4Addr;
}

pub struct NetworkToolsImpl {}

impl NetworkToolsImpl {
    pub fn new() -> Self {
        Self {}
    }
}

impl NetworkTools for NetworkToolsImpl {
    fn fetch_interface(&self, interface_name: &str) -> NetworkInterface {
        let all_interfaces = pnet::datalink::interfaces();
        for interface in all_interfaces {
            if interface.name == interface_name {
                return interface;
            }
        }

        panic!("Never supposed to happen!");
    }
    fn fetch_ipv4_address(&self, interface_name: &str) -> Option<std::net::Ipv4Addr> {
        let addrs = nix::ifaddrs::getifaddrs().unwrap();
        for ifaddr in addrs {
            // Right interface...
            if ifaddr.interface_name == interface_name {
                // Contains address
                if ifaddr.address.is_some() {
                    let address = ifaddr.address.unwrap();

                    // Contains hw address
                    if address.as_sockaddr_in().is_some() {
                        return Some(std::net::Ipv4Addr::from(
                            address.as_sockaddr_in().unwrap().ip().to_be_bytes(),
                        ));
                    }
                }
            }
        }

        return None;
    }

    fn fetch_ipv6_address(&self, interface_name: &str) -> Option<String> {
        let addrs = nix::ifaddrs::getifaddrs().unwrap();
        for ifaddr in addrs {
            // Right interface...
            if ifaddr.interface_name == interface_name {
                // Contains address
                if ifaddr.address.is_some() {
                    let address = ifaddr.address.unwrap();

                    // Contains hw address
                    if address.as_sockaddr_in6().is_some() {
                        let as_string = address.to_string();
                        return Some(as_string);
                    }
                }
            }
        }

        return None;
    }

    fn fetch_hardware_address(&self, interface_name: &str) -> Option<MacAddr> {
        let addrs = nix::ifaddrs::getifaddrs().unwrap();
        for ifaddr in addrs {
            // Right interface...
            if ifaddr.interface_name == interface_name {
                // Contains address
                if ifaddr.address.is_some() {
                    let address = ifaddr.address.unwrap();

                    // Contains hw address
                    if address.as_link_addr().is_some() {
                        let link_addr = address.as_link_addr();

                        // TODO: (@objectivecosta) remove unwraps!
                        let mac_addr = MacAddr::from(link_addr.unwrap().addr().unwrap());
                        return Some(mac_addr);
                    }
                }
            }
        }

        return None;
    }

    fn debug_iterate(&self) {
        let addrs = nix::ifaddrs::getifaddrs().unwrap();
        for ifaddr in addrs {
            match ifaddr.address {
                Some(address) => {
                    let _hw_address = address.as_link_addr();
                    println!("interface {} address {}", ifaddr.interface_name, address);
                }
                None => {
                    println!(
                        "interface {} with unsupported address family",
                        ifaddr.interface_name
                    );
                }
            }
        }
    }

    fn fetch_gateway_ip(&self) -> Ipv4Addr {
        let macos = "route -n get default | grep 'gateway' | awk '{print $2}'";
        let output = Command::new("sh")
            .arg("-c")
            .arg(macos)
            .output()
            .expect("failed to execute process");

        let hello = output.stdout;
        let as_str = String::from_utf8(hello).unwrap();
        let target_len = as_str.trim();

        let ipv4 = Ipv4Addr::from_str(target_len);

        return ipv4.unwrap();
    }
}
