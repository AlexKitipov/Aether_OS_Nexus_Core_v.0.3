// kernel/src/arch/x86_64/gdt.rs

#![allow(dead_code)]

use core::ptr::addr_of;

use lazy_static::lazy_static;
use x86_64::instructions::segmentation::{CS, Segment};
use x86_64::instructions::tables::load_tss;
use x86_64::structures::gdt::{Descriptor, GlobalDescriptorTable, SegmentSelector};
use x86_64::structures::tss::TaskStateSegment;
use x86_64::VirtAddr;

use crate::kprintln;

pub const DOUBLE_FAULT_IST_INDEX: u16 = 0;

const DOUBLE_FAULT_STACK_SIZE: usize = 4096 * 5;

struct Selectors {
    code_selector: SegmentSelector,
    tss_selector: SegmentSelector,
}

lazy_static! {
    static ref TSS: TaskStateSegment = {
        let mut tss = TaskStateSegment::new();
        static mut DOUBLE_FAULT_STACK: [u8; DOUBLE_FAULT_STACK_SIZE] = [0; DOUBLE_FAULT_STACK_SIZE];

        let stack_start = VirtAddr::from_ptr(addr_of!(DOUBLE_FAULT_STACK));
        let stack_end = stack_start + DOUBLE_FAULT_STACK_SIZE;
        tss.interrupt_stack_table[DOUBLE_FAULT_IST_INDEX as usize] = stack_end;
        tss
    };

    static ref GDT: (GlobalDescriptorTable, Selectors) = {
        let mut gdt = GlobalDescriptorTable::new();
        let code_selector = gdt.add_entry(Descriptor::kernel_code_segment());
        let tss_selector = gdt.add_entry(Descriptor::tss_segment(&TSS));

        (
            gdt,
            Selectors {
                code_selector,
                tss_selector,
            },
        )
    };
}

pub fn init() {
    kprintln!("[kernel] gdt: Initializing GDT and TSS...");

    GDT.0.load();
    unsafe {
        CS::set_reg(GDT.1.code_selector);
        load_tss(GDT.1.tss_selector);
    }

    kprintln!("[kernel] gdt: GDT and TSS loaded.");
}
