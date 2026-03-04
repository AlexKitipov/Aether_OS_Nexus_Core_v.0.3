// common/src/nexus_net_transport.rs

#![no_std]

extern crate alloc;
use alloc::vec::Vec;
use alloc::format;

use crate::cid::Cid;
use crate::swarm_engine::{SwarmTransport, SwarmError};
use crate::arp_dht::PeerInfo;
use libnexus_net::{NetClient, NetError};

// Temporary log function for V-Nodes
fn log(msg: &str) {
    unsafe {
        let res = crate::syscall::syscall3(
            crate::syscall::SYS_LOG,
            msg.as_ptr() as u64,
            msg.len() as u64,
            0 // arg3 is unused for SYS_LOG
        );
        if res != crate::syscall::SUCCESS { /* Handle log error, maybe panic or fall back */ }
    }
}

pub struct NexusNetTransport {
    net_client: NetClient,
    udp_socket_handle: u32, // Re-use a single UDP socket for all fetches
}

impl NexusNetTransport {
    pub fn new() -> Result<Self, NetError> {
        let mut net_client = NetClient::new();
        let udp_socket_handle = net_client.open_udp_socket(0)?; // Open an ephemeral UDP socket
        log(&alloc::format!("NexusNetTransport: Opened UDP socket with handle: {}", udp_socket_handle));
        Ok(NexusNetTransport {
            net_client,
            udp_socket_handle,
        })
    }
}

impl SwarmTransport for NexusNetTransport {
    fn fetch_chunk_from_peer(&self, peer: &PeerInfo, cid: Cid) -> Result<Vec<u8>, SwarmError> {
        log(&alloc::format!("NexusNetTransport: Fetching chunk {} from peer {}:{}",
            alloc::format!("{:?}", cid.as_bytes()), peer.ip_address[0], peer.port));

        // Serialize CID for sending
        let request_payload = postcard::to_allocvec(&cid).map_err(|_| SwarmError::NetworkError)?;

        // Send CID request to the peer over UDP
        self.net_client.send_to(
            self.udp_socket_handle,
            peer.ip_address,
            peer.port,
            request_payload
        ).map_err(|e| {
            log(&alloc::format!("NexusNetTransport: Failed to send request: {:?}", e));
            SwarmError::NetworkError
        })?;

        // Receive the response (chunk data)
        // This will block until a response is received or a timeout occurs
        // In a real system, we'd have a more robust async receive with timeouts
        let response_payload = self.net_client.recv(self.udp_socket_handle).map_err(|e| {
            log(&alloc::format!("NexusNetTransport: Failed to receive response: {:?}", e));
            SwarmError::NetworkError
        })?;

        // In a real scenario, the response payload would be verified and parsed to extract the chunk data.
        // For this sketch, we assume the response_payload IS the chunk data.
        log(&alloc::format!("NexusNetTransport: Received {} bytes for chunk {}", response_payload.len(), alloc::format!("{:?}", cid.as_bytes())));
        Ok(response_payload)
    }
}
