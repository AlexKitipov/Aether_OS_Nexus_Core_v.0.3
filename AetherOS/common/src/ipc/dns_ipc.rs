extern crate alloc;

use alloc::string::String;
#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

/// Represents a DNS query request from a client V-Node to the DNS Resolver V-Node.
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub enum DnsRequest {
    /// Request to resolve a hostname to an IPv4 address.
    ResolveHostname { hostname: String },
}

/// Represents a DNS response from the DNS Resolver V-Node to a client V-Node.
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub enum DnsResponse {
    /// Successful resolution of a hostname to an IPv4 address.
    ResolvedHostname { hostname: String, ip_address: [u8; 4] },
    /// Indicates that the hostname could not be resolved.
    NotFound { query: String },
    /// Indicates an error occurred during the resolution process.
    Error { message: String },
}
