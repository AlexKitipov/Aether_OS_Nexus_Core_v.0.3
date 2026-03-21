// kernel/src/arch/x86_64/paging.rs

#![allow(dead_code)] // Allow dead code for now as not all functions might be used immediately

use crate::kprintln;
use x86_64::registers::control::Cr3;

/// Initializes the paging system.
/// This includes setting up the initial page tables for the kernel's address space
/// (e.g., identity mapping for lower memory, higher-half mapping for kernel code/data).
pub fn init() {
    kprintln!("[kernel] paging: Initializing hardware paging...");

    // TODO: In a real implementation:
    // 1. Get the current physical frame allocator.
    // 2. Create a new recursive page table (or modify the bootloader-provided one).
    // 3. Map the kernel's physical memory to its higher-half virtual address.
    // 4. Identity map essential hardware registers (e.g., APIC, MMIO).
    // 5. Load the new page table base address into the CR3 register.
    // 6. Enable the PAE (Physical Address Extension) and PGE (Page Global Enable) bits in CR4 (if applicable).
    // 7. Enable paging by setting the PG bit in CR0.

    kprintln!("[kernel] paging: Higher-half kernel setup simulated.");
    kprintln!("[kernel] paging: Paging initialized (bootstrap stage).");
}

/// Returns the physical base address of the currently active kernel PML4 table.
pub fn get_kernel_pml4() -> u64 {
    let (level_4_table_frame, _) = Cr3::read();
    let pml4 = level_4_table_frame.start_address().as_u64();
    kprintln!("[kernel] paging: Active kernel PML4 at physical {:#x}.", pml4);
    pml4
}

/// Best-effort virtual-to-physical translation for bootstrap paths.
///
/// At this stage, we still use identity/direct-map semantics as a fallback.
/// Once full page-table walking is implemented, this function should traverse
/// PML4/PDPT/PD/PT entries and return the resolved physical address.
pub fn virt_to_phys(virt_addr: u64) -> u64 {
    virt_addr
}

/// Conceptually maps a virtual address to a physical address.
/// In a real system, this would involve modifying page table entries.
pub fn map(physical_address: usize, virtual_address: usize, flags: u64) {
    kprintln!("[kernel] paging: Mapping physical {:#x} to virtual {:#x} with flags {:#x} (conceptual).",
               physical_address, virtual_address, flags);
    // TODO: Implement actual page table entry modification.
}

/// Conceptually unmaps a virtual address.
/// In a real system, this would involve modifying page table entries.
pub fn unmap(virtual_address: usize) {
    kprintln!("[kernel] paging: Unmapping virtual {:#x} (conceptual).", virtual_address);
    // TODO: Implement actual page table entry modification and TLB invalidation.
}

/// Backward-compatible conceptual alias.
pub fn map_page(physical_address: usize, virtual_address: usize, flags: u64) {
    map(physical_address, virtual_address, flags);
}

/// Backward-compatible conceptual alias.
pub fn unmap_page(virtual_address: usize) {
    unmap(virtual_address);
}
