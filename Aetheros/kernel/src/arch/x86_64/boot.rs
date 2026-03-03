// kernel/src/arch/x86_64/boot.rs

#![allow(dead_code)] // Allow dead code for now as not all functions might be used immediately

use crate::kprintln;

/// A placeholder function that endlessly loops, effectively halting the CPU.
/// This is used for unrecoverable errors or at the end of kernel execution.
#[no_mangle]
pub extern "C" fn h_loop() -> ! {
    kprintln!("[kernel] boot: Entering infinite halt loop.");
    loop { 
        x86_64::instructions::hlt(); // Halt the CPU
    }
}

/// Conceptually sets up the CPU for long mode (64-bit mode).
/// In a real boot sequence, this would involve enabling PAE, setting up CR3,
/// and then jumping to 64-bit code.
pub fn long_mode_init() {
    kprintln!("[kernel] boot: Initializing long mode (conceptual)...");
    // In a real implementation, this would involve:
    // - Enabling PAE (Physical Address Extension) in CR4
    // - Loading a new Page Map Level 4 (PML4) table into CR3
    // - Enabling Long Mode by setting the LME bit in EFER MSR
    // - Enabling Paging by setting the PG bit in CR0
    // - Performing a far jump to a 64-bit code segment

    kprintln!("[kernel] boot: Long mode setup simulated.");
}

