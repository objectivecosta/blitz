use std::{
    collections::HashMap,
    net::{Ipv4Addr, SocketAddr, SocketAddrV4, Ipv6Addr, SocketAddrV6},
};

use async_trait::async_trait;
use dns_lookup::getnameinfo;
use tokio::task;

#[async_trait]
pub trait GetNameAddr {
    async fn get_from_address(&mut self, address: &Ipv4Addr) -> String;
    async fn get_from_address6(&mut self, address: &Ipv6Addr) -> String;
}

pub struct GetNameAddrImpl {
    cache: HashMap<String, String>,
}

impl GetNameAddrImpl {
    pub fn new() -> Self {
        Self {
            cache: HashMap::new(),
        }
    }
}

#[async_trait]
impl GetNameAddr for GetNameAddrImpl {
    async fn get_from_address(&mut self, address: &Ipv4Addr) -> String {
        if self.cache.contains_key(&address.to_string()) {
            let cached_value = self.cache.get(&address.to_string()).unwrap().to_string();
            // println!("Using cached value for host: {} = {}", dest.to_string(), cached_value);
            return cached_value;
        }

        let dest_sock_addr = SocketAddr::from(SocketAddrV4::new(*address, 80));
        let result = task::spawn_blocking(move || {
            let name_info: Result<(String, String), dns_lookup::LookupError> =
                getnameinfo(&dest_sock_addr, 0);
            return name_info.unwrap().0;
        })
        .await;

        let hostname = result.unwrap_or("unknown".to_owned());

        if hostname != address.to_string() {
            self.cache.insert(address.to_string(), hostname.clone());
        }

        return hostname;
    }

    async fn get_from_address6(&mut self, address: &Ipv6Addr) -> String {
      if self.cache.contains_key(&address.to_string()) {
          let cached_value = self.cache.get(&address.to_string()).unwrap().to_string();
          // println!("Using cached value for host: {} = {}", dest.to_string(), cached_value);
          return cached_value;
      }

      let dest_sock_addr = SocketAddr::from(SocketAddrV6::new(*address, 80, 0, 0));
      let result = task::spawn_blocking(move || {
          let name_info: Result<(String, String), dns_lookup::LookupError> =
              getnameinfo(&dest_sock_addr, 0);
          return name_info.unwrap().0;
      })
      .await;

      let hostname = result.unwrap_or("unknown".to_owned());

      if hostname != address.to_string() {
          self.cache.insert(address.to_string(), hostname.clone());
      }

      return hostname;
  }
}
