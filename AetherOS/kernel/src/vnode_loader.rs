// kernel/src/vnode_loader.rs

#![allow(dead_code)] // Allow dead code for now as not all functions might be used immediately

extern crate alloc;
use alloc::format;
use alloc::string::String;
use alloc::vec::Vec;

use crate::caps::Capability;
use crate::elf;
use crate::kprintln;
use crate::memory::page_allocator::PageAllocator;
use crate::task;

/// Stable identifier for immutable V-Node descriptors.
pub type VNodeId = u64;

/// High-level permission envelope attached to a V-Node image.
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

/// Immutable executable node descriptor.
#[derive(Debug, Clone)]
pub struct VNode {
    pub id: VNodeId,
    pub name: String,
    pub code: &'static [u8],
    pub entry: u64,
    pub permissions: Permissions,
}

/// Initializes the V-Node loader.
pub fn init() {
    kprintln!("[kernel] vnode_loader: Initializing V-Node loader (conceptual)...");
    kprintln!("[kernel] vnode_loader: V-Node loader initialized.");
}

/// Builds an immutable V-Node descriptor from an ELF image and static code bytes.
pub fn build_vnode_descriptor(
    id: VNodeId,
    name: &str,
    code: &'static [u8],
    entry: u64,
    permissions: Permissions,
) -> VNode {
    VNode {
        id,
        name: name.into(),
        code,
        entry,
        permissions,
    }
}

/// Creates a schedulable task out of an immutable V-Node descriptor.
pub fn spawn_vnode_task(vnode: &VNode, capabilities: Vec<Capability>) -> Result<(), String> {
    let stack_base = PageAllocator::allocate_page()
        .ok_or_else(|| format!("Failed to allocate user stack for V-Node '{}'.", vnode.name))?;
    let stack_top = stack_base + 4096u64;
    let address_space_root = crate::arch::x86_64::paging::get_kernel_pml4();

    // Immutable code bytes are represented in `VNode::code`; actual user mapping is
    // tracked separately and remains a TODO while pager integration lands.
    let _code_len = vnode.code.len();

    task::create_user_task(
        vnode.id,
        &vnode.name,
        capabilities,
        x86_64::VirtAddr::new(vnode.entry),
        stack_top,
        address_space_root,
    );

    kprintln!(
        "[kernel] vnode_loader: spawned V-Node '{}' as task {} (entry={:#x}).",
        vnode.name,
        vnode.id,
        vnode.entry
    );

    Ok(())
}

/// Conceptually loads a V-Node binary, parses its ELF, and creates a task for it.
pub fn load_vnode(vnode_name: &str, capabilities: Vec<Capability>) -> Result<(), String> {
    kprintln!("[kernel] vnode_loader: Loading V-Node: {}...", vnode_name);

    let vnode_path = format!("/initrd/{}.bin", vnode_name);
    kprintln!("[kernel] vnode_loader: Attempting to load from path: {}.", vnode_path);

    let elf_header = match elf::ElfLoader::load_elf(&vnode_path) {
        Ok(header) => header,
        Err(e) => {
            kprintln!("[kernel] vnode_loader: Failed to load ELF for {}: {}.", vnode_name, e);
            return Err(format!("Failed to load V-Node ELF: {}.", e));
        }
    };
    kprintln!(
        "[kernel] vnode_loader: ELF loaded for {}. Entry point: {:#x}.",
        vnode_name,
        elf_header.entry_point
    );

    // In this minimal runtime step we keep code bytes immutable and mapped as static metadata.
    let vnode = build_vnode_descriptor(
        1000 + vnode_name.as_bytes()[0] as u64,
        vnode_name,
        &[],
        elf_header.entry_point,
        Permissions::default(),
    );

    spawn_vnode_task(&vnode, capabilities)?;

    kprintln!("[kernel] vnode_loader: V-Node {} loaded successfully.", vnode_name);
    Ok(())
}
