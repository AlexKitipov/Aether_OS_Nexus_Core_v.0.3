// vnode/dns-resolver/src/main.rs

#![no_std]
#![no_main]

extern crate alloc;

use core::panic::PanicInfo;
use alloc::vec::Vec;
use alloc::collections::BTreeMap;
use alloc::format;
use alloc::string::{String, ToString};

use common::ipc::vnode::VNodeChannel;
use common::syscall::{syscall3, SYS_LOG, SUCCESS, SYS_TIME};
use common::ipc::socket_ipc::{SocketRequest, SocketResponse, SocketFd};
use common::ipc::dns_ipc::{DnsRequest, DnsResponse};

// Temporary log function for V-Nodes
fn log(msg: &str) {
    unsafe {
        let res = syscall3(
            SYS_LOG,
            msg.as_ptr() as u64,
            msg.len() as u64,
            0 // arg3 is unused for SYS_LOG
        );
        if res != SUCCESS { /* Handle log error, maybe panic or fall back */ }
    }
}

// Placeholder for DNS cache entry
struct DnsCacheEntry {
    ip_address: [u8; 4],
    expires_at_ms: u64,
}

// Main struct for the DNS Resolver V-Node logic
struct DnsResolver {
    client_chan: VNodeChannel,
    socket_chan: VNodeChannel,
    aetherfs_chan: VNodeChannel,
    dns_cache: BTreeMap<String, DnsCacheEntry>,
    dns_servers: Vec<[u8; 4]>,
    dns_socket_fd: SocketFd,
}

impl DnsResolver {
    fn new(client_chan_id: u32, socket_chan_id: u32, aetherfs_chan_id: u32) -> Self {
        let client_chan = VNodeChannel::new(client_chan_id);
        let mut socket_chan = VNodeChannel::new(socket_chan_id);
        let aetherfs_chan = VNodeChannel::new(aetherfs_chan_id);

        log("DNS Resolver: Initializing...");

        // Conceptual: Read /etc/network/resolv.conf for DNS server addresses.
        // For now, hardcode a dummy DNS server.
        let mut dns_servers = Vec::new();
        // Using Google DNS as a dummy, typically this would be configured by DHCP or admin.
        dns_servers.push([8, 8, 8, 8]);
        log(&alloc::format!("DNS Resolver: Using DNS server: {}.{}.{}.{}", dns_servers[0][0], dns_servers[0][1], dns_servers[0][2], dns_servers[0][3]));

        // Open a UDP socket with `socket-api` for sending DNS queries.
        let dns_socket_fd: SocketFd = match socket_chan.send_and_recv::<SocketRequest, SocketResponse>(&SocketRequest::Socket { domain: 2, ty: 2, protocol: 0 }) {
            Ok(SocketResponse::Success(fd)) => {
                log(&alloc::format!("DNS Resolver: Opened UDP socket with fd: {}.", fd));
                fd as SocketFd
            },
            Ok(SocketResponse::Error(err_code, msg)) => {
                log(&alloc::format!("DNS Resolver: Failed to open UDP socket with socket-api. Error {}: {}. Fatal error.", err_code, msg));
                panic!("Failed to open UDP socket");
            },
            _ => {
                log("DNS Resolver: Unexpected response from socket-api when opening UDP socket. Fatal error.");
                panic!("Unexpected socket-api response");
            }
        };

        Self {
            client_chan,
            socket_chan,
            aetherfs_chan,
            dns_cache: BTreeMap::new(),
            dns_servers,
            dns_socket_fd,
        }
    }

    // This function encapsulates the network lookup logic for a hostname
    fn perform_network_lookup(&mut self, hostname: &String, current_time_ms: u64) -> DnsResponse {
        log(&alloc::format!("DNS Resolver: Performing network lookup for {}.", hostname));

        // For now, let's simulate a successful lookup for "example.com" and a failure for others.
        // In a real system, we'd construct a proper DNS query packet (e.g., using a DNS library).
        let dns_query_payload = alloc::format!("DNS_QUERY:{}", hostname).as_bytes().to_vec();

        // Use the first configured DNS server.
        let dns_server_ip = self.dns_servers[0];
        const DNS_PORT: u16 = 53; // Standard DNS port

        // 1. "Connect" the UDP socket to the remote DNS server. For UDP, this just sets the default peer.
        match self.socket_chan.send_and_recv::<SocketRequest, SocketResponse>(&SocketRequest::Connect { fd: self.dns_socket_fd, addr: dns_server_ip, port: DNS_PORT }) {
            Ok(SocketResponse::Success(_)) => log(&alloc::format!("DNS Resolver: UDP socket {} connected to {}:{}", self.dns_socket_fd, dns_server_ip[0], DNS_PORT)),
            Ok(SocketResponse::Error(err_code, msg)) => {
                log(&alloc::format!("DNS Resolver: Failed to connect UDP socket to DNS server. Error {}: {}.", err_code, msg));
                return DnsResponse::Error { message: "Failed to set remote DNS server".to_string() };
            },
            _ => {
                log("DNS Resolver: Unexpected response during UDP connect to DNS server.");
                return DnsResponse::Error { message: "Unexpected response during UDP connect".to_string() };
            }
        }

        // 2. Send the simulated DNS query packet over UDP.
        match self.socket_chan.send_and_recv::<SocketRequest, SocketResponse>(&SocketRequest::Send { fd: self.dns_socket_fd, data: dns_query_payload }) {
            Ok(SocketResponse::Success(bytes_sent)) => log(&alloc::format!("DNS Resolver: Sent {} bytes DNS query for {}.", bytes_sent, hostname)),
            Ok(SocketResponse::Error(err_code, msg)) => {
                log(&alloc::format!("DNS Resolver: Failed to send DNS query for {}. Error {}: {}.", hostname, err_code, msg));
                return DnsResponse::Error { message: "Failed to send DNS query".to_string() };
            },
            _ => {
                log("DNS Resolver: Unexpected response during DNS query send.");
                return DnsResponse::Error { message: "Unexpected response during DNS query send".to_string() };
            }
        }

        // 3. Receive the simulated DNS response.
        // In a real system, there would be a timeout here.
        match self.socket_chan.send_and_recv::<SocketRequest, SocketResponse>(&SocketRequest::Recv { fd: self.dns_socket_fd, len: 512 }) {
            Ok(SocketResponse::Data(response_payload)) => {
                // Conceptual: Parse the DNS response.
                let response_str = alloc::string::String::from_utf8_lossy(&response_payload);
                log(&alloc::format!("DNS Resolver: Received DNS response: {}.", response_str));

                if response_str.contains("IP:192.0.2.1") && hostname == "example.com" {
                    let ip_addr = [192, 0, 2, 1]; // Dummy IP for example.com
                    let expires_at_ms = current_time_ms + 60_000; // Cache for 60 seconds
                    self.dns_cache.insert(hostname.clone(), DnsCacheEntry { ip_address: ip_addr, expires_at_ms });
                    log(&alloc::format!("DNS Resolver: Resolved {} to {}.{}.{}.{} (cached).", hostname, ip_addr[0], ip_addr[1], ip_addr[2], ip_addr[3]));
                    DnsResponse::ResolvedHostname { hostname: hostname.clone(), ip_address: ip_addr }
                } else if response_str.contains("NOT_FOUND") {
                    log(&alloc::format!("DNS Resolver: Hostname {} not found by external server.", hostname));
                    DnsResponse::NotFound { query: hostname.clone() }
                } else {
                    log(&alloc::format!("DNS Resolver: Unknown response format or unexpected result for {}.", hostname));
                    DnsResponse::Error { message: alloc::format!("Unknown DNS response for {}.", hostname) }
                }
            },
            Ok(SocketResponse::Error(err_code, msg)) => {
                log(&alloc::format!("DNS Resolver: Failed to receive DNS response for {}. Error {}: {}.", hostname, err_code, msg));
                DnsResponse::Error { message: "Failed to receive DNS response".to_string() }
            },
            _ => {
                log("DNS Resolver: Unexpected response during DNS response receive.");
                DnsResponse::Error { message: "Unexpected response during DNS response receive".to_string() };
            }
        }
    }

    fn run_loop(&mut self) -> ! {
        log("DNS Resolver: Entering main event loop.");
        loop {
            let current_time_ms = unsafe { syscall3(SYS_TIME, 0, 0, 0) * 10 }; // Assuming 1 tick = 10 ms

            // 1. Process incoming DNS queries from client V-Nodes
            if let Ok(Some(req_data)) = self.client_chan.recv_non_blocking() {
                if let Ok(request) = postcard::from_bytes::<DnsRequest>(&req_data) {
                    log(&alloc::format!("DNS Resolver: Received DnsRequest: {:?}.", request));

                    let response = match request {
                        DnsRequest::ResolveHostname { hostname } => {
                            // Check cache first
                            if let Some(entry) = self.dns_cache.get(&hostname) {
                                if current_time_ms < entry.expires_at_ms {
                                    log(&alloc::format!("DNS Resolver: Cache hit for {}: {}.{}.{}.{}.", hostname, entry.ip_address[0], entry.ip_address[1], entry.ip_address[2], entry.ip_address[3]));
                                    DnsResponse::ResolvedHostname { hostname: hostname.clone(), ip_address: entry.ip_address }
                                } else {
                                    log(&alloc::format!("DNS Resolver: Cache expired for {}.", hostname));
                                    self.dns_cache.remove(&hostname);
                                    // Fall through to network lookup
                                    self.perform_network_lookup(&hostname, current_time_ms)
                                }
                            } else {
                                log(&alloc::format!("DNS Resolver: Cache miss for {}, performing network lookup.", hostname));
                                self.perform_network_lookup(&hostname, current_time_ms)
                            }
                        },
                    };
                    self.client_chan.send(&response).unwrap_or_else(|_| log("DNS Resolver: Failed to send response to client."));
                } else {
                    log("DNS Resolver: Failed to deserialize DnsRequest from client.");
                }
            }

            // Yield to other V-Nodes to prevent busy-waiting
            unsafe { syscall3(SYS_TIME, 0, 0, 0); } // This will cause a context switch
        }
    }
}

#[no_mangle]
pub extern "C" fn _start() -> ! {
    // Assuming channel IDs:
    // 5 for DNS Resolver Service client requests
    // 4 for Socket API Service
    // 6 for AetherFS (for config reads, currently conceptual)
    let mut dns_resolver = DnsResolver::new(5, 4, 6);
    dns_resolver.run_loop();
}

#[panic_handler]
pub extern "C" fn panic(info: &PanicInfo) -> ! {
    log(&alloc::format!("DNS Resolver V-Node panicked! Info: {:?}.", info));
    // In a production system, this might trigger a system-wide error handler or reboot.
    // For now, it enters an infinite loop to prevent further execution.
    loop {}
}
