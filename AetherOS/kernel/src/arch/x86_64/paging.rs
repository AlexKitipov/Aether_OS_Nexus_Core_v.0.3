// kernel/src/arch/x86_64/paging.rs

#![allow(dead_code)] // Allow dead code for now as not all functions might be used immediately

use crate::kprintln;
use super::boot;

/// Initializes the paging system.
/// This includes setting up the initial page tables for the kernel's address space
/// (e.g., identity mapping for lower memory, higher-half mapping for kernel code/data).
pub fn init() {
    kprintln!("[kernel] paging: Initializing paging (conceptual)...");

    // TODO: In a real implementation:
    // 1. Get the current physical frame allocator.
    // 2. Create a new recursive page table (or modify the bootloader-provided one).
    // 3. Map the kernel's physical memory to its higher-half virtual address.
    // 4. Identity map essential hardware registers (e.g., APIC, MMIO).
    // 5. Load the new page table base address into the CR3 register.
    // 6. Enable the PAE (Physical Address Extension) and PGE (Page Global Enable) bits in CR4 (if applicable).
    // 7. Enable paging by setting the PG bit in CR0.

    kprintln!("[kernel] paging: Higher-half kernel setup simulated.");
    kprintln!("[kernel] paging: Paging conceptually enabled.");
}

/// Returns the physical base address of the currently active kernel PML4 table.
///
/// For now this uses the active CR3 value as a pragmatic bridge between
/// conceptual paging setup and boot-time long mode orchestration.
pub fn get_kernel_pml4() -> u64 {
    let pml4 = boot::read_cr3() & !0xFFF;
    kprintln!("[kernel] paging: Active kernel PML4 at physical {:#x}.", pml4);
    pml4
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
