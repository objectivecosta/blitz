pub trait NetworkTools {
    fn debug_iterate(&self);
    fn fetch_hardware_address(&self, interface_name: &str) -> String;
    fn fetch_ipv4_address(&self, interface_name: &str) -> String;
    fn fetch_ipv6_address(&self, interface_name: &str) -> Option<String>;
}

pub struct NetworkToolsImpl {}

impl NetworkToolsImpl {
    pub fn new() -> Self {
        Self {}
    }
}

impl NetworkTools for NetworkToolsImpl {
    fn fetch_ipv4_address(&self, interface_name: &str) -> String {
        let addrs = nix::ifaddrs::getifaddrs().unwrap();
        for ifaddr in addrs {
            // Right interface...
            if ifaddr.interface_name == interface_name {
                // Contains address
                if ifaddr.address.is_some() {
                    let address = ifaddr.address.unwrap();

                    // Contains hw address
                    if address.as_sockaddr_in().is_some() {
                        let as_string = address.to_string();
                        return as_string;
                    }
                }
            }
        }

        return "none".to_owned();
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
    
    fn fetch_hardware_address(&self, interface_name: &str) -> String {
        let addrs = nix::ifaddrs::getifaddrs().unwrap();
        for ifaddr in addrs {
            // Right interface...
            if ifaddr.interface_name == interface_name {
                // Contains address
                if ifaddr.address.is_some() {
                    let address = ifaddr.address.unwrap();

                    // Contains hw address
                    if address.as_link_addr().is_some() {
                        let as_string = address.to_string();
                        return as_string;
                    }
                }
            }
        }

        return "none".to_owned();
    }

    fn debug_iterate(&self) {
        let addrs = nix::ifaddrs::getifaddrs().unwrap();
        for ifaddr in addrs {
            match ifaddr.address {
                Some(address) => {
                    let hw_address = address.as_link_addr();
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
}
