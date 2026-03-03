# DNS Resolver (svc://dns-resolver)

## Overview

The `dns-resolver` V-Node provides a centralized DNS resolution service for all other V-Nodes within AetherOS. It handles hostname-to-IP address lookups, communicates with external DNS servers (conceptually via `svc://socket-api`), caches results, and exposes a clean IPC interface for client applications. This ensures that DNS queries are handled efficiently and consistently across the system.

## IPC Protocol

Communication with the `dns-resolver` V-Node occurs via IPC, using the `DnsRequest` and `DnsResponse` enums defined in `src/ipc/dns_ipc.rs`.

### DnsRequest Enum (Client -> dns-resolver)

Client V-Nodes send these requests to `svc://dns-resolver` to perform DNS lookups.

```rust
#[derive(Debug, Serialize, Deserialize)]
pub enum DnsRequest {
    /// Request to resolve a hostname to an IPv4 address.
    ResolveHostname { hostname: String },
    /// Request to reverse resolve an IPv4 address to a hostname.
    // ReverseResolveIp { ip_address: [u8; 4] },
}
```

**Parameters:**

*   `hostname`: A `String` representing the hostname to be resolved (e.g., "example.com").

### DnsResponse Enum (dns-resolver -> Client)

`svc://dns-resolver` sends these responses back to the client V-Node after processing a `DnsRequest`.

```rust
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
```

**Return Values:**

*   `ResolvedHostname { hostname: String, ip_address: [u8; 4] }`: Indicates a successful resolution, returning the original hostname and its corresponding IPv4 address.
*   `NotFound { query: String }`: The requested hostname could not be resolved.
*   `Error { message: String }`: An internal error occurred during the resolution process, with a descriptive message.

## Functionality

The `dns-resolver` V-Node performs the following key functions:

1.  **Request Handling**: Listens for `DnsRequest` messages on its dedicated IPC channel.
2.  **DNS Cache**: Maintains an in-memory cache of resolved hostnames and their corresponding IP addresses. Entries have a configurable Time-To-Live (TTL).
3.  **`/etc/network/resolv.conf`**: Conceptually reads this file to discover the IP addresses of upstream DNS servers.
4.  **UDP Client (via Socket API)**: Uses `svc://socket-api` to open a UDP socket and send DNS queries to configured upstream DNS servers.
5.  **Response Parsing**: Parses DNS responses received from upstream servers.
6.  **Error Handling**: Catches network errors, timeouts, or invalid responses and reports them back to the client.

## Usage Examples

### Example: Resolving a Hostname

```rust
// Pseudocode for client V-Node wanting to resolve a hostname

let mut dns_resolver_chan = VNodeChannel::new(5); // IPC Channel to svc://dns-resolver

// Request to resolve a hostname
let request = DnsRequest::ResolveHostname { hostname: String::from("example.com") };
match dns_resolver_chan.send_and_recv::<DnsRequest, DnsResponse>(&request) {
    Ok(DnsResponse::ResolvedHostname { hostname, ip_address }) => {
        log!("Resolved {}: {}.{}.{}.{}", hostname, ip_address[0], ip_address[1], ip_address[2], ip_address[3]);
    },
    Ok(DnsResponse::NotFound { query }) => {
        log!("Hostname {} not found.", query);
    },
    Ok(DnsResponse::Error { message }) => {
        log!("DNS resolution error: {}", message);
    },
    _ => log!("Unexpected response from DNS Resolver"),
}
```

This documentation outlines the critical role of the DNS Resolver in providing network services and how other V-Nodes can interact with it securely and efficiently through its well-defined IPC interface.
