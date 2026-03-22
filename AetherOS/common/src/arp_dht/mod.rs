use serde::{Deserialize, Serialize};

pub struct NodeId(pub [u8; 32]);

/// Minimal peer routing metadata shared across transport + swarm layers.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct PeerInfo {
    pub ip_address: [u8; 4],
    pub port: u16,
    pub vnode_id: u32,
}

pub enum DhtValue {
    Manifest(super::examples::Manifest),
}

pub struct InMemoryDht;

impl InMemoryDht {
    pub fn new() -> Self { InMemoryDht }
    pub fn store(&self, _key: [u8; 32], _value: DhtValue) {}
}
