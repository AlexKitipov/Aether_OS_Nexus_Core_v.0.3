use alloc::vec::Vec;
use core::cmp::min;

#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

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
    #[cfg(feature = "serde")]
    use serde::{Deserialize, Serialize};

    use crate::arp_dht::PeerInfo;

    /// Cross-peer search request emitted by shell/UI services.
    #[derive(Debug, Clone, PartialEq, Eq)]
    #[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
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
    #[derive(Debug, Clone, PartialEq, Eq)]
    #[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
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

/// Stable node identity in the Aether Swarm (public key bytes).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct NodeId(pub [u8; 32]);

/// Private key bytes used for local signing/handshake state.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct NodeSecret(pub [u8; 32]);

/// Capability flags that define what a remote node is allowed to do.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct NodeCapability {
    pub allow_vnode_exec: bool,
    pub allow_snapshot_sync: bool,
    pub allow_remote_fs_read: bool,
}

impl NodeCapability {
    #[must_use]
    pub const fn full_federation() -> Self {
        Self {
            allow_vnode_exec: true,
            allow_snapshot_sync: true,
            allow_remote_fs_read: true,
        }
    }

    #[must_use]
    pub const fn restricted() -> Self {
        Self {
            allow_vnode_exec: false,
            allow_snapshot_sync: true,
            allow_remote_fs_read: false,
        }
    }
}

/// Snapshot digest used by anti-entropy gossip.
pub type SnapshotHash = [u8; 32];
/// Immutable blob/tree hash in distributed AetherFS replication.
pub type ObjectHash = [u8; 32];
/// Node load metric used by scheduling/placement hints.
pub type NodeLoad = u16;
/// Deterministic V-Node identifier.
pub type VNodeId = u64;

/// Cluster-level health state used for distributed runtime supervision.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub enum NodeHealth {
    Healthy,
    Degraded,
    Unresponsive,
}

/// Runtime telemetry payload exchanged between swarm nodes.
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct NodeTelemetry {
    pub node_id: [u8; 32],
    pub snapshot_hash: SnapshotHash,
    pub health: NodeHealth,
    pub cpu_usage: f32,
    pub mem_used: u64,
    pub mem_free: u64,
    pub vnode_count: u32,
    pub available_vnodes: Vec<VNodeId>,
}

/// Bootstrap + identity metadata broadcast during discovery.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct NodeInfo {
    pub node_id: NodeId,
    pub transport: PeerInfo,
    pub capabilities: NodeCapability,
}

/// Immutable V-Node payload transferred for remote execution.
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct VNodeImage {
    pub vnode_id: VNodeId,
    pub snapshot_hash: SnapshotHash,
    pub bytes: Vec<u8>,
}

/// ASP (Aether Swarm Protocol) wire message set.
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub enum SwarmMessage {
    Hello(NodeInfo),
    Gossip { snapshot_hash: SnapshotHash, node_load: NodeLoad },
    Telemetry(NodeTelemetry),
    RequestSnapshotObject(ObjectHash),
    SendSnapshotObject { hash: ObjectHash, blob: Vec<u8> },
    RequestVNode(VNodeId),
    SendVNode(VNodeImage),
    Ping(u64),
    Pong(u64),
}

#[cfg(feature = "serde")]
impl SwarmMessage {
    /// Serialize to compact deterministic binary envelope.
    pub fn encode(&self) -> Result<Vec<u8>, SwarmError> {
        postcard::to_allocvec(self).map_err(|_| SwarmError::InvalidRequest)
    }

    /// Deserialize from compact deterministic binary envelope.
    pub fn decode(bytes: &[u8]) -> Result<Self, SwarmError> {
        postcard::from_bytes(bytes).map_err(|_| SwarmError::InvalidResponse)
    }
}

/// Canonical discovery endpoints used by ASP bootstrap.
pub struct DiscoveryEndpoints;

impl DiscoveryEndpoints {
    pub const SWARM_PORT: u16 = 7777;
    pub const LAN_BROADCAST: [u8; 4] = [255, 255, 255, 255];
    pub const DEFAULT_BOOTSTRAP_HOST: &'static str = "swarm.aetheros.net";
}

/// Minimal secure-channel state used to protect ASP payloads.
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct SecureChannel {
    pub key: [u8; 32],
    pub nonce: u64,
}

impl SecureChannel {
    #[must_use]
    pub const fn new(key: [u8; 32]) -> Self {
        Self { key, nonce: 0 }
    }

    /// Lightweight placeholder key-derivation from node secret and peer id.
    #[must_use]
    pub fn derive_from_identity(secret: &NodeSecret, peer: &NodeId) -> Self {
        let mut key = [0u8; 32];
        for (idx, slot) in key.iter_mut().enumerate() {
            *slot = secret.0[idx] ^ peer.0[31 - idx];
        }
        Self { key, nonce: 0 }
    }

    /// XOR stream used as deterministic stand-in for encrypted envelopes.
    #[must_use]
    pub fn seal(&mut self, plaintext: &[u8]) -> Vec<u8> {
        let mut out = Vec::with_capacity(plaintext.len());
        for (idx, b) in plaintext.iter().enumerate() {
            let k = self.key[(idx + (self.nonce as usize % 32)) % 32];
            out.push(*b ^ k);
        }
        self.nonce = self.nonce.wrapping_add(1);
        out
    }

    #[must_use]
    pub fn open(&mut self, ciphertext: &[u8]) -> Vec<u8> {
        // Symmetric with `seal` because XOR stream is reversible.
        self.seal(ciphertext)
    }
}

/// Remote execution federation policy for known nodes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct FederationPolicy {
    pub local: NodeCapability,
}

impl FederationPolicy {
    #[must_use]
    pub const fn can_serve_vnode(&self, remote: NodeCapability) -> bool {
        self.local.allow_vnode_exec && remote.allow_vnode_exec
    }

    #[must_use]
    pub const fn can_sync_snapshots(&self, remote: NodeCapability) -> bool {
        self.local.allow_snapshot_sync && remote.allow_snapshot_sync
    }
}

/// Anti-entropy planner for selecting missing snapshot objects.
pub struct SnapshotSyncPlanner;

impl SnapshotSyncPlanner {
    #[must_use]
    pub fn missing_objects(local: &[ObjectHash], remote: &[ObjectHash]) -> Vec<ObjectHash> {
        let mut missing = Vec::new();
        for remote_hash in remote {
            if !local.iter().any(|local_hash| local_hash == remote_hash) {
                missing.push(*remote_hash);
            }
        }
        missing
    }

    /// Small helper to chunk object requests and keep UDP payloads bounded.
    #[must_use]
    pub fn plan_batches(missing: &[ObjectHash], batch_size: usize) -> Vec<Vec<ObjectHash>> {
        let mut batches = Vec::new();
        if batch_size == 0 {
            return batches;
        }

        let mut cursor = 0;
        while cursor < missing.len() {
            let next = min(cursor + batch_size, missing.len());
            batches.push(missing[cursor..next].to_vec());
            cursor = next;
        }
        batches
    }
}
