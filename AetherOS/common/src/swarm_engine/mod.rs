use alloc::vec::Vec;

use crate::arp_dht::PeerInfo;

/// Error codes returned by the Swarm Engine abstraction.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SwarmError {
    NetworkError,
    RoutingNotFound,
    InvalidRequest,
    InvalidResponse,
}

/// Transport contract used by the Swarm Engine.
pub trait SwarmTransport {
    /// Fetch a content chunk from a concrete peer.
    fn fetch_chunk_from_peer(&self, peer: &PeerInfo, cid: [u8; 32]) -> Result<Vec<u8>, SwarmError>;
}

/// Core swarm orchestrator that delegates network I/O to a transport.
pub struct SwarmEngine<T: SwarmTransport> {
    transport: T,
}

impl<T: SwarmTransport> SwarmEngine<T> {
    #[must_use]
    pub const fn new(transport: T) -> Self {
        Self { transport }
    }

    pub fn fetch_chunk_from_peer(&self, peer: &PeerInfo, cid: [u8; 32]) -> Result<Vec<u8>, SwarmError> {
        self.transport.fetch_chunk_from_peer(peer, cid)
    }
}

pub mod global_search {
    extern crate alloc;

    use alloc::{string::String, vec::Vec};
    use serde::{Deserialize, Serialize};

    use crate::arp_dht::PeerInfo;

    /// Cross-peer search request emitted by shell/UI services.
    #[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
    pub struct SearchRequest {
        pub query: String,
        /// Optional maximum number of results expected by caller.
        pub max_results: u16,
        /// If true, local index should be preferred before fan-out.
        pub prefer_local_first: bool,
    }

    impl SearchRequest {
        #[must_use]
        pub fn new(query: impl Into<String>) -> Self {
            Self {
                query: query.into(),
                max_results: 20,
                prefer_local_first: true,
            }
        }
    }

    /// Basic search result primitive used for IPC/VFS projections.
    #[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
    pub struct SearchResult {
        pub cid: [u8; 32],
        pub owner: PeerInfo,
        pub score: u16,
        pub path_hint: String,
    }

    /// Stateless façade for global search orchestration.
    pub struct GlobalSearchService;

    impl GlobalSearchService {
        #[must_use]
        pub const fn new() -> Self {
            Self
        }

        /// Placeholder planner that will later query local DHT + remote peers.
        #[must_use]
        pub fn select_peers<'a>(&self, known_peers: &'a [PeerInfo], _request: &SearchRequest) -> Vec<&'a PeerInfo> {
            known_peers.iter().collect()
        }
    }
}
