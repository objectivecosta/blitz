# `blitz`

## What is `blitz`

Blitz is a ~~firewall that works by ARP spoofing/NPD spoofing devices in your network to allow for packet inspection and filtering~~ 
learning project for me to become more familiar with Rust.
It is written in Rust using `tokio`, `libpnet` and others.

## Stage

This is in the proof of concept stage/hobby. DO *NOT* use it for production applications.

## Checklist

### ARP/~~NDP~~

- [x] ~~Send ARP spoofing to target devices pretending to be the router~~
- [x] ~~Send ARP spoofing to router pretending to be the target devices~~
- [x] ~~Can perform ARP queries~~
- [ ] ~~Send NDP spoofing to target devices pretending to be the router~~
- [ ] ~~Send NDP spoofing to router pretending to be the target devices~~

### Routing

- [ ] Implements DHCP server for IPv4
- [ ] Forwards packets upstream

### Packet Inspection

- [x] Does reverse DNS of packet's source/destination to find traffic flows
- [x] Can log tx/rx to a specific host
- [ ] Can filter packets based on IP ranges
- [ ] Can filter packets based on specific hostnames
- [ ] Can filter packets based on RegEx on hostnames
- [x] Can create log files of traffic data

### API

- [ ] Provides HTTP API for management
