#![allow(dead_code)]

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::vec::Vec;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

pub const SNAPSHOT_FORMAT_VERSION: u32 = 1;
const WIRE_HEADER_SIZE: usize = 4 + 32 + 4;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct SnapshotHeader {
    pub version: u32,
    pub id: u64,
    pub created_at: u64,
    pub prev_id: Option<u64>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct VNodeState {
    pub vnode_id: u64,
    pub image_hash: [u8; 32],
    pub caps_hash: [u8; 32],
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Snapshot {
    pub header: SnapshotHeader,
    pub vnodes: Vec<VNodeState>,
    pub fs_root_hash: [u8; 32],
}

#[derive(Debug)]
pub enum SnapshotError {
    Encode(postcard::Error),
    Decode(postcard::Error),
    VersionMismatch { expected: u32, got: u32 },
    InvalidHash,
    InvalidWireFormat,
}

#[derive(Debug, Clone, Copy)]
pub struct SnapshotWire<'a> {
    pub version: u32,
    pub hash: [u8; 32],
    pub payload: &'a [u8],
}

pub trait SnapshotStorage {
    fn load_latest(&self) -> Option<Vec<u8>>;
    fn load_by_id(&self, id: u64) -> Option<Vec<u8>>;
    fn store(&mut self, id: u64, data: &[u8]) -> Result<(), SnapshotError>;
}

#[derive(Default)]
pub struct InMemorySnapshotStorage {
    inner: BTreeMap<u64, Vec<u8>>,
}

impl InMemorySnapshotStorage {
    pub fn new() -> Self {
        Self { inner: BTreeMap::new() }
    }
}

impl SnapshotStorage for InMemorySnapshotStorage {
    fn load_latest(&self) -> Option<Vec<u8>> {
        self.inner.keys().max().and_then(|k| self.inner.get(k).cloned())
    }

    fn load_by_id(&self, id: u64) -> Option<Vec<u8>> {
        self.inner.get(&id).cloned()
    }

    fn store(&mut self, id: u64, data: &[u8]) -> Result<(), SnapshotError> {
        self.inner.insert(id, data.to_vec());
        Ok(())
    }
}

pub fn encode_snapshot(snap: &Snapshot) -> Result<Vec<u8>, SnapshotError> {
    postcard::to_allocvec(snap).map_err(SnapshotError::Encode)
}

pub fn decode_snapshot(data: &[u8]) -> Result<Snapshot, SnapshotError> {
    postcard::from_bytes(data).map_err(SnapshotError::Decode)
}

pub fn wrap_snapshot(bytes: &[u8]) -> SnapshotWire<'_> {
    let hash = sha2_256(bytes);
    SnapshotWire {
        version: SNAPSHOT_FORMAT_VERSION,
        hash,
        payload: bytes,
    }
}

pub fn encode_wire(wire: &SnapshotWire<'_>) -> Vec<u8> {
    let mut out = Vec::with_capacity(WIRE_HEADER_SIZE + wire.payload.len());
    out.extend_from_slice(&wire.version.to_le_bytes());
    out.extend_from_slice(&wire.hash);
    out.extend_from_slice(&(wire.payload.len() as u32).to_le_bytes());
    out.extend_from_slice(wire.payload);
    out
}

pub fn decode_wire(bytes: &[u8]) -> Result<SnapshotWire<'_>, SnapshotError> {
    if bytes.len() < WIRE_HEADER_SIZE {
        return Err(SnapshotError::InvalidWireFormat);
    }

    let version = u32::from_le_bytes(bytes[0..4].try_into().map_err(|_| SnapshotError::InvalidWireFormat)?);

    let mut hash = [0u8; 32];
    hash.copy_from_slice(&bytes[4..36]);

    let payload_len =
        u32::from_le_bytes(bytes[36..40].try_into().map_err(|_| SnapshotError::InvalidWireFormat)?) as usize;

    if bytes.len() != WIRE_HEADER_SIZE + payload_len {
        return Err(SnapshotError::InvalidWireFormat);
    }

    Ok(SnapshotWire {
        version,
        hash,
        payload: &bytes[WIRE_HEADER_SIZE..],
    })
}

pub fn verify_and_decode_wire(wire: &SnapshotWire<'_>) -> Result<Snapshot, SnapshotError> {
    if wire.version != SNAPSHOT_FORMAT_VERSION {
        return Err(SnapshotError::VersionMismatch {
            expected: SNAPSHOT_FORMAT_VERSION,
            got: wire.version,
        });
    }

    let calc = sha2_256(wire.payload);
    if calc != wire.hash {
        return Err(SnapshotError::InvalidHash);
    }

    decode_snapshot(wire.payload)
}

pub fn store_snapshot<S: SnapshotStorage>(storage: &mut S, snapshot: &Snapshot) -> Result<(), SnapshotError> {
    let payload = encode_snapshot(snapshot)?;
    let wire = wrap_snapshot(&payload);
    let wire_bytes = encode_wire(&wire);
    storage.store(snapshot.header.id, &wire_bytes)
}

pub fn load_latest_snapshot<S: SnapshotStorage>(storage: &S) -> Result<Option<Snapshot>, SnapshotError> {
    let bytes = match storage.load_latest() {
        Some(bytes) => bytes,
        None => return Ok(None),
    };

    let wire = decode_wire(&bytes)?;
    verify_and_decode_wire(&wire).map(Some)
}

pub fn load_snapshot_by_id<S: SnapshotStorage>(storage: &S, id: u64) -> Result<Option<Snapshot>, SnapshotError> {
    let bytes = match storage.load_by_id(id) {
        Some(bytes) => bytes,
        None => return Ok(None),
    };

    let wire = decode_wire(&bytes)?;
    verify_and_decode_wire(&wire).map(Some)
}

fn sha2_256(bytes: &[u8]) -> [u8; 32] {
    let mut hasher = Sha256::new();
    hasher.update(bytes);

    let digest = hasher.finalize();
    let mut hash = [0u8; 32];
    hash.copy_from_slice(&digest);
    hash
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_snapshot(id: u64, prev_id: Option<u64>) -> Snapshot {
        Snapshot {
            header: SnapshotHeader {
                version: SNAPSHOT_FORMAT_VERSION,
                id,
                created_at: 1_711_111_111,
                prev_id,
            },
            vnodes: vec![VNodeState {
                vnode_id: 7,
                image_hash: [0xAA; 32],
                caps_hash: [0xBB; 32],
            }],
            fs_root_hash: [0xCC; 32],
        }
    }

    #[test]
    fn snapshot_roundtrip_via_wire_storage() {
        let mut storage = InMemorySnapshotStorage::new();
        let snap = sample_snapshot(1, None);

        store_snapshot(&mut storage, &snap).unwrap();
        let loaded = load_latest_snapshot(&storage).unwrap().unwrap();

        assert_eq!(loaded.header.id, snap.header.id);
        assert_eq!(loaded.header.prev_id, snap.header.prev_id);
        assert_eq!(loaded.vnodes.len(), 1);
        assert_eq!(loaded.vnodes[0].image_hash, [0xAA; 32]);
    }

    #[test]
    fn wire_hash_verification_fails_on_tamper() {
        let snap = sample_snapshot(2, Some(1));
        let payload = encode_snapshot(&snap).unwrap();
        let wire = wrap_snapshot(&payload);
        let mut encoded = encode_wire(&wire);

        let last = encoded.len() - 1;
        encoded[last] ^= 0x01;

        let decoded_wire = decode_wire(&encoded).unwrap();
        assert!(matches!(verify_and_decode_wire(&decoded_wire), Err(SnapshotError::InvalidHash)));
    }
}
