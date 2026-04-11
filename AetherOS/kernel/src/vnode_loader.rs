#![allow(dead_code)]

extern crate alloc;

use alloc::format;
use alloc::string::String;
use alloc::vec::Vec;
// use postcard::to_allocvec;
use sha2::{Digest, Sha256};
use spin::Mutex;

use crate::aetherfs::{self, FsCapability, FsRights, Hash};
use crate::caps::Capability;
use crate::elf;
use crate::kprintln;
use crate::memory::page_allocator::PageAllocator;
use crate::task;

pub type VNodeId = u64;

#[derive(Debug, Clone)]
pub struct Permissions {
    pub can_syscall: bool,
    pub can_ipc: bool,
    pub can_io: bool,
}

impl Default for Permissions {
    fn default() -> Self {
        Self {
            can_syscall: true,
            can_ipc: true,
            can_io: false,
        }
    }
}

#[derive(Debug, Clone)]
pub struct VNode {
    pub id: VNodeId,
    pub name: String,
    pub image_hash: Hash,
    pub entry: u64,
    pub permissions: Permissions,
    pub fs_capability: FsCapability,
}

#[derive(Debug, Clone)]
struct ManagedVNode {
    id: VNodeId,
    image_hash: Hash,
    capabilities: Vec<Capability>,
}

impl ManagedVNode {
    fn capability_hash(&self) -> [u8; 32] {
        let encoded = Vec::new(); // TODO: serialize capabilities
        sha2_256(&encoded)
    }
}

static VNODE_MANAGER: Mutex<Vec<ManagedVNode>> = Mutex::new(Vec::new());

pub fn init() {
    kprintln!("[kernel] vnode_loader: Initializing immutable V-Node loader...");
    kprintln!("[kernel] vnode_loader: Ready.");
}

pub fn build_vnode_descriptor(
    id: VNodeId,
    name: &str,
    image_hash: Hash,
    entry: u64,
    permissions: Permissions,
    fs_capability: FsCapability,
) -> VNode {
    VNode {
        id,
        name: name.into(),
        image_hash,
        entry,
        permissions,
        fs_capability,
    }
}

pub fn check_fs_cap(vnode: &VNode, path: &str, right: FsRights) -> bool {
    if right == FsRights::ReadWrite && vnode.fs_capability.rights != FsRights::ReadWrite {
        return false;
    }

    aetherfs::fs_resolve_path(vnode.fs_capability.root, path).is_some()
}

pub fn spawn_vnode_task(vnode: &VNode, capabilities: Vec<Capability>) -> Result<(), String> {
    let managed_capabilities = capabilities.clone();
    let stack_base = PageAllocator::allocate_page()
        .ok_or_else(|| format!("Failed to allocate user stack for V-Node '{}'.", vnode.name))?;
    let stack_top = stack_base + 4096u64;
    let address_space_root = crate::arch::x86_64::paging::get_kernel_pml4();

    task::create_user_task(
        vnode.id,
        &vnode.name,
        capabilities,
        x86_64::VirtAddr::new(vnode.entry),
        stack_top,
        address_space_root,
    );

    kprintln!(
        "[kernel] vnode_loader: spawned V-Node '{}' as task {} (entry={:#x}, image={:02x?}).",
        vnode.name,
        vnode.id,
        vnode.entry,
        vnode.image_hash.0
    );

    VNODE_MANAGER.lock().push(ManagedVNode {
        id: vnode.id,
        image_hash: vnode.image_hash,
        capabilities: managed_capabilities,
    });

    Ok(())
}

pub fn load_vnode(vnode_name: &str, capabilities: Vec<Capability>) -> Result<(), String> {
    kprintln!("[kernel] vnode_loader: Loading V-Node '{}'.", vnode_name);

    let boot_snapshot = aetherfs::load_snapshot(aetherfs::BOOT_SNAPSHOT_HASH)
        .ok_or_else(|| String::from("Boot snapshot not available"))?;

    let vnode_path = format!("/initrd/{}.bin", vnode_name);
    let image_hash = aetherfs::fs_resolve_path(boot_snapshot.root, &vnode_path)
        .ok_or_else(|| format!("V-Node image not found at '{}'", vnode_path))?;
    let image = aetherfs::fs_read(image_hash)
        .ok_or_else(|| format!("V-Node image hash {:02x?} is not readable", image_hash.0))?;
    let elf_header = elf::ElfLoader::parse_elf_bytes(&image)?;

    let vnode = build_vnode_descriptor(
        1000 + vnode_name.as_bytes()[0] as u64,
        vnode_name,
        image_hash,
        elf_header.entry_point,
        Permissions::default(),
        FsCapability {
            root: boot_snapshot.root,
            rights: FsRights::ReadOnly,
        },
    );

    if !check_fs_cap(&vnode, &vnode_path, FsRights::ReadOnly) {
        return Err(format!("FS capability check failed for {}", vnode_path));
    }

    spawn_vnode_task(&vnode, capabilities)?;

    kprintln!("[kernel] vnode_loader: V-Node '{}' loaded from immutable storage.", vnode_name);
    Ok(())
}

pub fn snapshot_vnode_states() -> Vec<crate::snapshot_engine::VNodeState> {
    VNODE_MANAGER
        .lock()
        .iter()
        .map(|vnode| crate::snapshot_engine::VNodeState {
            vnode_id: vnode.id,
            image_hash: vnode.image_hash.0,
            caps_hash: vnode.capability_hash(),
        })
        .collect()
}

pub fn spawn_from_snapshot(vnode: &crate::snapshot_engine::VNodeState) -> Result<(), String> {
    let image_hash = Hash(vnode.image_hash);
    let image = aetherfs::fs_read(image_hash)
        .ok_or_else(|| format!("V-Node image hash {:02x?} is not readable", vnode.image_hash))?;
    let elf_header = elf::ElfLoader::parse_elf_bytes(&image)?;

    let current = aetherfs::current_snapshot().ok_or_else(|| String::from("AetherFS has no active snapshot"))?;
    let descriptor = build_vnode_descriptor(
        vnode.vnode_id,
        "restored-vnode",
        image_hash,
        elf_header.entry_point,
        Permissions::default(),
        FsCapability {
            root: current.root,
            rights: FsRights::ReadOnly,
        },
    );

    spawn_vnode_task(&descriptor, Vec::new())
}

fn sha2_256(bytes: &[u8]) -> [u8; 32] {
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    let digest = hasher.finalize();
    let mut hash = [0u8; 32];
    hash.copy_from_slice(&digest);
    hash
}
