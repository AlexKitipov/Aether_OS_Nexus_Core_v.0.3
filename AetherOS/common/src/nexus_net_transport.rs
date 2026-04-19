// common/src/nexus_net_transport.rs

extern crate alloc;
use alloc::vec::Vec;
use alloc::format;

use crate::swarm_engine::{SwarmTransport, SwarmError};
use crate::arp_dht::PeerInfo;
use libnexus_net::{NetClient, NetError};

// Temporary log function for V-Nodes
fn log(msg: &str) {
    let res = crate::syscall::syscall_log(msg);
    if res != crate::syscall::SUCCESS {
        // best-effort logging path; intentionally ignore errors here
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
        log(&format!("NexusNetTransport: Opened UDP socket with handle: {}", udp_socket_handle));
        Ok(NexusNetTransport {
            net_client,
            udp_socket_handle,
        })
    }
}

#[cfg(feature = "serde")]
impl SwarmTransport for NexusNetTransport {
    #[cfg(feature = "serde")]
    fn fetch_chunk_from_peer(&mut self, peer: &PeerInfo, cid: [u8; 32]) -> Result<Vec<u8>, SwarmError> {
        log(&format!("NexusNetTransport: Fetching chunk {:?} from peer {:?}:{}",
            &cid, peer.ip_address, peer.port));

        // Serialize CID for sending
        let request_payload = postcard::to_allocvec(&cid).map_err(|_| SwarmError::InvalidRequest)?;

        // Send CID request to the peer over UDP
        self.net_client.send_to(
            self.udp_socket_handle,
            peer.ip_address,
            peer.port,
            &request_payload // Corrected: pass as reference
        ).map_err(|e| {
            log(&format!("NexusNetTransport: Failed to send request: {:?}", e));
            SwarmError::NetworkError
        })?;

        // Receive the response (chunk data)
        // This will block until a response is received or a timeout occurs
        // In a real system, we'd have a more robust async receive with timeouts
        let response_payload = self.net_client.recv(self.udp_socket_handle).map_err(|e| {
            log(&format!("NexusNetTransport: Failed to receive response: {:?}", e));
            SwarmError::NetworkError
        })?;

        // In a real scenario, the response payload would be verified and parsed to extract the chunk data.
        // For this sketch, we assume the response_payload IS the chunk data.
        log(&format!("NexusNetTransport: Received {} bytes for chunk {:?}", response_payload.len(), &cid));
        Ok(response_payload)
    }
}