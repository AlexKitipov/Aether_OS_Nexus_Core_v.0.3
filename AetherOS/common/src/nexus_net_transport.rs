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
    udp_socket_handle: u32,
}

impl NexusNetTransport {
    pub fn new() -> Result<Self, NetError> {
        let mut net_client = NetClient::new();
        let udp_socket_handle = net_client.open_udp_socket(0)?;
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
        log(&format!(
            "NexusNetTransport: Fetching chunk {:?} from peer {:?}:{}",
            &cid, peer.ip_address, peer.port
        ));

        let request_payload =
            postcard::to_allocvec(&cid).map_err(|_| SwarmError::InvalidRequest)?;

        self.net_client
            .send_to(
                peer.ip_address,
                peer.port,
                &request_payload,
            )
            .map_err(|e| {
                log(&format!("NexusNetTransport: Failed to send request: {:?}", e));
                SwarmError::NetworkError
            })?;

        let response_payload = self
            .net_client
            .recv(self.udp_socket_handle)
            .map_err(|e| {
                log(&format!(
                    "NexusNetTransport: Failed to receive response: {:?}",
                    e
                ));
                SwarmError::NetworkError
            })?;

        log(&format!(
            "NexusNetTransport: Received {} bytes for chunk {:?}",
            response_payload.len(),
            &cid
        ));

        Ok(response_payload)
    }
}

