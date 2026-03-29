#![allow(dead_code)]

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::string::{String, ToString};
use alloc::vec;
use alloc::vec::Vec;
use sha2::{Digest, Sha256};
use spin::Mutex;

use crate::kprintln;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Hash(pub [u8; 32]);

impl Hash {
    pub const fn zero() -> Self {
        Self([0; 32])
    }
}

#[derive(Debug, Clone)]
pub struct FsEntry {
    pub name: String,
    pub hash: Hash,
}

#[derive(Debug, Clone)]
pub enum FsObject {
    Blob(Vec<u8>),
    Tree(Vec<FsEntry>),
    VNodeImage(Vec<u8>),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Snapshot {
    pub root: Hash,
    pub parent: Option<Hash>,
    pub timestamp: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FsRights {
    ReadOnly,
    ReadWrite,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FsCapability {
    pub root: Hash,
    pub rights: FsRights,
}

#[derive(Debug, Default)]
struct AetherFs {
    objects: BTreeMap<Hash, FsObject>,
    snapshots: BTreeMap<Hash, Snapshot>,
    head: Option<Hash>,
}

impl AetherFs {
    fn put_object(&mut self, object: FsObject) -> Hash {
        let hash = hash_object(&object);
        self.objects.entry(hash).or_insert(object);
        hash
    }

    fn get_object(&self, hash: Hash) -> Option<&FsObject> {
        self.objects.get(&hash)
    }

    fn commit_snapshot(&mut self, root: Hash, timestamp: u64) -> Hash {
        let snapshot = Snapshot {
            root,
            parent: self.head,
            timestamp,
        };
        let snapshot_hash = hash_snapshot(&snapshot);
        self.snapshots.insert(snapshot_hash, snapshot);
        self.head = Some(snapshot_hash);
        snapshot_hash
    }
}

static FS: Mutex<AetherFs> = Mutex::new(AetherFs {
    objects: BTreeMap::new(),
    snapshots: BTreeMap::new(),
    head: None,
});

pub const BOOT_SNAPSHOT_HASH: Hash = Hash::zero();

pub fn init() {
    let mut fs = FS.lock();
    if fs.head.is_some() {
        return;
    }

    let init_elf = demo_elf_image(0x401000);
    let init_hash = fs.put_object(FsObject::VNodeImage(init_elf));
    let manifest_hash = fs.put_object(FsObject::Blob(br#"{"name":"init-service"}"#.to_vec()));

    let initrd_tree_hash = fs.put_object(FsObject::Tree(vec![
        FsEntry {
            name: "init-service.bin".to_string(),
            hash: init_hash,
        },
        FsEntry {
            name: "manifest.json".to_string(),
            hash: manifest_hash,
        },
    ]));

    let root_hash = fs.put_object(FsObject::Tree(vec![FsEntry {
        name: "initrd".to_string(),
        hash: initrd_tree_hash,
    }]));

    let snapshot_hash = fs.commit_snapshot(root_hash, 0);
    kprintln!(
        "[kernel] aetherfs: initialized in-memory immutable store (snapshot={:02x?}).",
        snapshot_hash.0
    );
}

pub fn load_snapshot(hash: Hash) -> Option<Snapshot> {
    let fs = FS.lock();
    if hash == BOOT_SNAPSHOT_HASH {
        return fs.head.and_then(|head| fs.snapshots.get(&head).copied());
    }
    fs.snapshots.get(&hash).copied()
}

pub fn current_snapshot() -> Option<Snapshot> {
    let fs = FS.lock();
    fs.head.and_then(|head| fs.snapshots.get(&head).copied())
}

pub fn fs_read(hash: Hash) -> Option<Vec<u8>> {
    let fs = FS.lock();
    match fs.get_object(hash) {
        Some(FsObject::Blob(data)) | Some(FsObject::VNodeImage(data)) => Some(data.clone()),
        _ => None,
    }
}

pub fn fs_list(tree: Hash) -> Vec<FsEntry> {
    let fs = FS.lock();
    match fs.get_object(tree) {
        Some(FsObject::Tree(entries)) => entries.clone(),
        _ => Vec::new(),
    }
}

pub fn fs_resolve_path(root: Hash, path: &str) -> Option<Hash> {
    let fs = FS.lock();
    let mut current = root;

    for part in path.split('/').filter(|segment| !segment.is_empty()) {
        let next = match fs.get_object(current) {
            Some(FsObject::Tree(entries)) => entries.iter().find(|entry| entry.name == part).map(|entry| entry.hash),
            _ => None,
        }?;
        current = next;
    }

    Some(current)
}

pub fn read_file(path: &str) -> Result<Vec<u8>, String> {
    let snapshot = current_snapshot().ok_or_else(|| "AetherFS has no active snapshot".to_string())?;
    let hash = fs_resolve_path(snapshot.root, path)
        .ok_or_else(|| alloc::format!("Path '{}' not found in active snapshot", path))?;
    fs_read(hash).ok_or_else(|| alloc::format!("Path '{}' does not reference a readable blob", path))
}

pub fn write_file(_path: &str, _data: &[u8]) -> Result<(), String> {
    Err("AetherFS is immutable; write requires creating a new snapshot".to_string())
}

pub fn object_hash(object: &FsObject) -> Hash {
    hash_object(object)
}

fn hash_object(object: &FsObject) -> Hash {
    let mut hasher = Sha256::new();
    match object {
        FsObject::Blob(data) => {
            hasher.update([0x01]);
            hasher.update((data.len() as u64).to_le_bytes());
            hasher.update(data);
        }
        FsObject::Tree(entries) => {
            hasher.update([0x02]);
            hasher.update((entries.len() as u64).to_le_bytes());
            for entry in entries {
                hasher.update((entry.name.len() as u64).to_le_bytes());
                hasher.update(entry.name.as_bytes());
                hasher.update(entry.hash.0);
            }
        }
        FsObject::VNodeImage(data) => {
            hasher.update([0x03]);
            hasher.update((data.len() as u64).to_le_bytes());
            hasher.update(data);
        }
    }

    let digest = hasher.finalize();
    let mut bytes = [0u8; 32];
    bytes.copy_from_slice(&digest);
    Hash(bytes)
}

fn hash_snapshot(snapshot: &Snapshot) -> Hash {
    let mut hasher = Sha256::new();
    hasher.update(snapshot.root.0);
    match snapshot.parent {
        Some(parent) => hasher.update(parent.0),
        None => hasher.update([0u8; 32]),
    }
    hasher.update(snapshot.timestamp.to_le_bytes());

    let digest = hasher.finalize();
    let mut bytes = [0u8; 32];
    bytes.copy_from_slice(&digest);
    Hash(bytes)
}

fn demo_elf_image(entry: u64) -> Vec<u8> {
    let mut elf = vec![0u8; 64];
    elf[0..4].copy_from_slice(b"\x7FELF");
    elf[4] = 2;
    elf[5] = 1;
    elf[6] = 1;
    elf[16..18].copy_from_slice(&2u16.to_le_bytes());
    elf[18..20].copy_from_slice(&0x3Eu16.to_le_bytes());
    elf[20..24].copy_from_slice(&1u32.to_le_bytes());
    elf[24..32].copy_from_slice(&entry.to_le_bytes());
    elf[32..40].copy_from_slice(&64u64.to_le_bytes());
    elf[52..54].copy_from_slice(&64u16.to_le_bytes());
    elf[54..56].copy_from_slice(&56u16.to_le_bytes());
    elf[56..58].copy_from_slice(&1u16.to_le_bytes());
    elf
}
