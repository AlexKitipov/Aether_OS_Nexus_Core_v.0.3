
// src/ipc/dns_ipc.rs

#![no_std]

extern crate alloc;
use alloc::vec::Vec;
use alloc::string::String;

use serde::{Deserialize, Serialize};

/// Represents a DNS query request from a client V-Node to the DNS Resolver V-Node.
#[derive(Debug, Serialize, Deserialize)]
pub enum DnsRequest {
    /// Request to resolve a hostname to an IPv4 address.
    ResolveHostname { hostname: String },
    /// Request to reverse resolve an IPv4 address to a hostname.
    // ReverseResolveIp { ip_address: [u8; 4] },
}

/// Represents a DNS response from the DNS Resolver V-Node to a client V-Node.
#[derive(Debug, Serialize, Deserialize)]
pub enum DnsResponse {
    /// Successful resolution of a hostname to an IPv4 address.
    ResolvedHostname { hostname: String, ip_address: [u8; 4] },
    /// Successful reverse resolution of an IP address to a hostname.
    // ResolvedIp { ip_address: [u8; 4], hostname: String },
    /// Indicates that the hostname or IP could not be resolved.
    NotFound { query: String },
    /// Indicates an error occurred during the resolution process.
    Error { message: String },
}
