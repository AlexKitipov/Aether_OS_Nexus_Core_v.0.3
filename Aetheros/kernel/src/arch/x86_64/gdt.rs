// kernel/src/arch/x86_64/gdt.rs

#![allow(dead_code)] // Allow dead code for now as not all functions might be used immediately

use x86_64::VirtAddr;
use x86_64::instructions::segmentation::{CS, Segment};
use x86_64::instructions::tables::lgdt;
use x86_64::structures::gdt::{Descriptor, SegmentSelector, GlobalDescriptorTable};
use crate::kprintln;

/// Define our Global Descriptor Table
/// The GDT contains entries for kernel code and data segments.
static mut GDT: GlobalDescriptorTable = GlobalDescriptorTable::new();

/// Define our segment selectors
/// These are used to load the segment registers after the GDT is loaded.
/// The `CS` selector is special and requires a far jump.
static mut KERNEL_CODE_SELECTOR: SegmentSelector;
static mut KERNEL_DATA_SELECTOR: SegmentSelector;

/// Initializes the GDT and loads it into the CPU.
/// Also reloads segment registers with the new selectors.
pub fn init() {
    // SAFETY: We are writing to static mut variables, but this is only called once at boot.
    unsafe {
        kprintln!("[kernel] gdt: Initializing GDT...");

        // Add kernel code and data segments to the GDT
        KERNEL_CODE_SELECTOR = GDT.add_entry(Descriptor::kernel_code_segment());
        KERNEL_DATA_SELECTOR = GDT.add_entry(Descriptor::kernel_data_segment());

        // Load the GDT into the CPU
        lgdt(&GDT.base_linear_addr(), GDT.len() as u16);
        kprintln!("[kernel] gdt: GDT loaded. Base: {:#x}, Length: {}.", GDT.base_linear_addr().as_u64(), GDT.len());

        // Reload segment registers
        // Reloading CS requires a far jump, which is handled by a helper function.
        CS::set_reg(KERNEL_CODE_SELECTOR);
        kprintln!("[kernel] gdt: CS reloaded with selector {:#?}.", KERNEL_CODE_SELECTOR);
        
        // Reload other segment registers (DS, ES, FS, GS, SS)
        // For 64-bit mode, these are often zeroed out or set to the data segment selector.
        // The x86_64 crate's SegmentSelector allows setting them.
        x86_64::instructions::segmentation::DS::set_reg(KERNEL_DATA_SELECTOR);
        x86_64::instructions::segmentation::ES::set_reg(KERNEL_DATA_SELECTOR);
        x86_64::instructions::segmentation::FS::set_reg(KERNEL_DATA_SELECTOR);
        x86_64::instructions::segmentation::GS::set_reg(KERNEL_DATA_SELECTOR);
        x86_64::instructions::segmentation::SS::set_reg(KERNEL_DATA_SELECTOR);

        kprintln!("[kernel] gdt: Segment registers reloaded.");
    }
}

