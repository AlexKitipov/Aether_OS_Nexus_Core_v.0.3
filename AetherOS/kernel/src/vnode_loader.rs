// kernel/src/vnode_loader.rs

#![allow(dead_code)] // Allow dead code for now as not all functions might be used immediately

extern crate alloc;
use alloc::string::{String, ToString};
use alloc::vec::Vec;
use crate::kprintln;
use crate::elf;
use crate::task;
use crate::caps::Capability;

/// Initializes the V-Node loader.
pub fn init() {
    kprintln!("[kernel] vnode_loader: Initializing V-Node loader (conceptual)...");
    // Any setup required for V-Node loading infrastructure.
    kprintln!("[kernel] vnode_loader: V-Node loader initialized.");
}

/// Conceptually loads a V-Node binary, parses its ELF, and creates a task for it.
/// 
/// In a real system, this would involve:
/// - Allocating memory for the V-Node's address space.
/// - Copying ELF segments into the V-Node's memory.
/// - Setting up V-Node specific capabilities based on its manifest.
/// - Creating a new CPU context (task) for the V-Node.
pub fn load_vnode(vnode_name: &str, capabilities: Vec<Capability>) -> Result<(), String> {
    kprintln!("[kernel] vnode_loader: Loading V-Node: {}...", vnode_name);

    // 1. Construct path for the V-Node's binary.
    let vnode_path = format!("/initrd/{}.bin", vnode_name);
    kprintln!("[kernel] vnode_loader: Attempting to load from path: {}.", vnode_path);

    // 2. Use ElfLoader to simulate loading the binary.
    let elf_header = match elf::ElfLoader::load_elf(&vnode_path) {
        Ok(header) => header,
        Err(e) => {
            kprintln!("[kernel] vnode_loader: Failed to load ELF for {}: {}.", vnode_name, e);
            return Err(format!("Failed to load V-Node ELF: {}.", e));
        }
    };
    kprintln!("[kernel] vnode_loader: ELF loaded for {}. Entry point: {:#x}.", vnode_name, elf_header.entry_point);

    // 3. Create a new task (V-Node) for the loaded ELF.
    // Assign a dummy task ID for now. In a real system, task IDs would be managed centrally.
    let dummy_task_id = 1000 + vnode_name.as_bytes()[0] as u64; // Simple dummy ID
    task::create_task(dummy_task_id, vnode_name, capabilities);
    kprintln!("[kernel] vnode_loader: Task created for V-Node {} (ID: {}).", vnode_name, dummy_task_id);

    // TODO: In a real system, the V-Node's entry point would be set up as the task's starting point.
    // For this conceptual stub, we just simulate the loading process.

    kprintln!("[kernel] vnode_loader: V-Node {} loaded successfully.", vnode_name);
    Ok(())
}
