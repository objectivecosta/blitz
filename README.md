# `blitz`

## What is `blitz`

Blitz is a firewall that works by ARPSpoofing/NPDSpoofing devices in your network to allow for packet inspection and filtering. 
It is written in Rust using tokio:io.

## Stage

This is in the proof of concept stage. DO *NOT* use it for production applications.

## Checklist

### ARP/NDP

[x] Send ARP spoofing to target devices pretending to be the router
[] Send ARP spoofing to router pretending to be the target devices
[] Send NDP spoofing to target devices pretending to be the router
[] Send NDP spoofing to router pretending to be the target devices

### Packet Inspection

[x] Does reverse DNS of packet's source/destination to find traffic flows
[x] Can log tx/rx to a specific host
[] Can filter packets based on IP ranges
[] Can filter packets based on specific hostnames
[] Can filter packets based on RegEx on hostnames
[] Can create log files of traffic data

### API

[] Provides HTTP API for management
