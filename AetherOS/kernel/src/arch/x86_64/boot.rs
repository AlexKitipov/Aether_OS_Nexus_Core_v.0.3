// kernel/src/arch/x86_64/boot.rs

#![allow(dead_code)]

use core::arch::asm;

use super::{gdt, idt, paging};
use crate::interrupts;
use crate::kprintln;

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

const CPUID_EXTENDED_FUNCTION_INFO: u32 = 0x8000_0001;
const CPUID_EXTENDED_MAX_LEAF: u32 = 0x8000_0000;
const CPUID_EDX_LONG_MODE: u32 = 1 << 29;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BootError {
    UnalignedPageTableAddress(u64),
    InvalidPhysicalAddress(u64),
    FailedToEnterLongMode(u64),
    CpuDoesNotSupportLongMode,
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

/// Checks whether the CPU supports long mode via CPUID.
pub fn check_long_mode_support() -> Result<(), BootError> {
    let max_extended_leaf: u32;
    unsafe {
        asm!(
            "cpuid",
            inout("eax") CPUID_EXTENDED_MAX_LEAF => max_extended_leaf,
            lateout("ebx") _,
            lateout("ecx") _,
            lateout("edx") _,
            options(nomem, nostack)
        );
    }

    if max_extended_leaf < CPUID_EXTENDED_FUNCTION_INFO {
        return Err(BootError::CpuDoesNotSupportLongMode);
    }

    let mut eax = CPUID_EXTENDED_FUNCTION_INFO;
    let edx_features: u32;
    unsafe {
        asm!(
            "cpuid",
            inout("eax") eax,
            lateout("ebx") _,
            lateout("ecx") _,
            lateout("edx") edx_features,
            options(nomem, nostack)
        );
    }

    if (edx_features & CPUID_EDX_LONG_MODE) == 0 {
        return Err(BootError::CpuDoesNotSupportLongMode);
    }

    Ok(())
}

/// Backward-compatible alias.
pub fn check_cpu_support() -> Result<(), BootError> {
    check_long_mode_support()
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

/// Full architecture initialization pipeline for booting into kernel mode.
pub fn architecture_init(config: LongModeConfig) -> Result<(), BootError> {
    gdt::init();
    idt::init();
    check_long_mode_support()?;
    long_mode_init(config)?;
    interrupts::init();

    kprintln!("[kernel] boot: Architecture initialized.");
    Ok(())
}

/// Architecture boot entry point for setup orchestration with an explicit
/// PML4 physical address provided by the bootloader/bootstrap stage.
pub fn entry_point_with_pml4(pml4_phys_addr: u64) {
    kprintln!("[kernel] boot: Starting bootstrap sequence.");
    let config = LongModeConfig::new(pml4_phys_addr);

    match architecture_init(config) {
        Ok(()) => {
            kprintln!("[kernel] boot: System is ready.");
            kernel_main();
        }
        Err(error) => {
            kprintln!("[kernel] boot ERROR: {:?}", error);
            h_loop();
        }
    }
}

/// Backward-compatible bootstrap entry point that discovers the active PML4.
pub fn entry_point() {
    paging::init();
    let pml4_addr = paging::get_kernel_pml4();
    entry_point_with_pml4(pml4_addr);
}

/// Minimal kernel runtime hand-off after successful bootstrap.
pub fn kernel_main() {
    x86_64::instructions::interrupts::enable();
    kprintln!("[kernel] boot: Interrupts enabled. Main loop starting...");
}
