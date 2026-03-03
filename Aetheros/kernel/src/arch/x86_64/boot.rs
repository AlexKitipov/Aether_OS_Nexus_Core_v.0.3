// kernel/src/arch/x86_64/boot.rs

#![allow(dead_code)]

use core::arch::asm;

use crate::kprintln;
use super::{gdt, idt, interrupts};

/// IA32_EFER MSR index.
pub const IA32_EFER: u32 = 0xC000_0080;

const CR0_PE: u64 = 1 << 0;
const CR0_WP: u64 = 1 << 16;
const CR0_PG: u64 = 1 << 31;

const CR4_PAE: u64 = 1 << 5;
const CR4_PGE: u64 = 1 << 7;

const EFER_LME: u64 = 1 << 8;
const EFER_LMA: u64 = 1 << 10;
const EFER_NXE: u64 = 1 << 11;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BootError {
    UnalignedPageTableAddress(u64),
    InvalidPhysicalAddress(u64),
    FailedToEnterLongMode(u64),
}

#[derive(Debug, Clone, Copy)]
pub struct LongModeConfig {
    /// Physical address of the PML4 page table.
    pub pml4_phys_addr: u64,
    /// Enable global pages for kernel mappings.
    pub enable_global_pages: bool,
    /// Enable NX bit support.
    pub enable_nxe: bool,
    /// Keep write-protect enabled in supervisor mode.
    pub keep_wp: bool,
}

impl LongModeConfig {
    pub const fn new(pml4_phys_addr: u64) -> Self {
        Self {
            pml4_phys_addr,
            enable_global_pages: true,
            enable_nxe: true,
            keep_wp: true,
        }
    }
}

/// A placeholder function that endlessly loops, effectively halting the CPU.
/// This is used for unrecoverable errors or at the end of kernel execution.
#[no_mangle]
pub extern "C" fn h_loop() -> ! {
    kprintln!("[kernel] boot: Entering infinite halt loop.");
    loop {
        x86_64::instructions::hlt();
    }
}

/// Reads an MSR value.
#[inline]
pub fn read_msr(msr: u32) -> u64 {
    let high: u32;
    let low: u32;
    unsafe {
        asm!(
            "rdmsr",
            in("ecx") msr,
            out("edx") high,
            out("eax") low,
            options(nomem, nostack, preserves_flags)
        );
    }
    ((high as u64) << 32) | (low as u64)
}

/// Writes an MSR value.
#[inline]
pub fn write_msr(msr: u32, value: u64) {
    let high = (value >> 32) as u32;
    let low = value as u32;
    unsafe {
        asm!(
            "wrmsr",
            in("ecx") msr,
            in("edx") high,
            in("eax") low,
            options(nomem, nostack, preserves_flags)
        );
    }
}

#[inline]
pub fn read_cr0() -> u64 {
    let value: u64;
    unsafe {
        asm!("mov {}, cr0", out(reg) value, options(nomem, nostack, preserves_flags));
    }
    value
}

#[inline]
pub fn write_cr0(value: u64) {
    unsafe {
        asm!("mov cr0, {}", in(reg) value, options(nomem, nostack, preserves_flags));
    }
}

#[inline]
pub fn read_cr3() -> u64 {
    let value: u64;
    unsafe {
        asm!("mov {}, cr3", out(reg) value, options(nomem, nostack, preserves_flags));
    }
    value
}

#[inline]
pub fn write_cr3(value: u64) {
    unsafe {
        asm!("mov cr3, {}", in(reg) value, options(nomem, nostack, preserves_flags));
    }
}

#[inline]
pub fn read_cr4() -> u64 {
    let value: u64;
    unsafe {
        asm!("mov {}, cr4", out(reg) value, options(nomem, nostack, preserves_flags));
    }
    value
}

#[inline]
pub fn write_cr4(value: u64) {
    unsafe {
        asm!("mov cr4, {}", in(reg) value, options(nomem, nostack, preserves_flags));
    }
}

#[inline]
pub fn set_cr0_bits(mask: u64) {
    write_cr0(read_cr0() | mask);
}

#[inline]
pub fn set_cr4_bits(mask: u64) {
    write_cr4(read_cr4() | mask);
}

#[inline]
fn validate_pml4_addr(pml4_phys_addr: u64) -> Result<(), BootError> {
    if pml4_phys_addr & 0xFFF != 0 {
        return Err(BootError::UnalignedPageTableAddress(pml4_phys_addr));
    }

    // Ensure bits 63:52 are clear (canonical physical address under 52-bit PA width).
    if pml4_phys_addr & 0xFFF0_0000_0000_0000 != 0 {
        return Err(BootError::InvalidPhysicalAddress(pml4_phys_addr));
    }

    Ok(())
}

/// Performs long mode activation:
/// 1. Enable PAE in CR4.
/// 2. Load PML4 physical base into CR3.
/// 3. Set LME (and optional NXE) in EFER MSR.
/// 4. Enable PE/PG in CR0 (and optional WP).
pub fn long_mode_init(config: LongModeConfig) -> Result<(), BootError> {
    validate_pml4_addr(config.pml4_phys_addr)?;

    kprintln!("[kernel] boot: Initializing x86_64 long mode.");

    let mut cr4 = read_cr4() | CR4_PAE;
    if config.enable_global_pages {
        cr4 |= CR4_PGE;
    }
    write_cr4(cr4);

    write_cr3(config.pml4_phys_addr);

    let mut efer = read_msr(IA32_EFER) | EFER_LME;
    if config.enable_nxe {
        efer |= EFER_NXE;
    }
    write_msr(IA32_EFER, efer);

    let mut cr0 = read_cr0() | CR0_PE | CR0_PG;
    if config.keep_wp {
        cr0 |= CR0_WP;
    }
    write_cr0(cr0);

    let efer_after = read_msr(IA32_EFER);
    if (efer_after & EFER_LMA) == 0 {
        return Err(BootError::FailedToEnterLongMode(efer_after));
    }

    kprintln!("[kernel] boot: Long mode enabled successfully.");
    Ok(())
}

/// Architecture boot entry point for basic kernel setup orchestration.
pub fn entry_point() {
    kernel_main();
}

/// Minimal kernel bootstrap sequence for descriptor tables and interrupts.
pub fn kernel_main() {
    gdt::init();
    idt::init();
    interrupts::init();
    x86_64::instructions::interrupts::enable();
}
