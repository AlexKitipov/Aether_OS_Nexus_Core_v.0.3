// kernel/src/arch/x86_64/gdt.rs

#![allow(dead_code)]

use crate::kprintln;
use x86_64::{
    instructions::{
        segmentation::{Segment, CS, DS, ES, FS, GS, SS},
        tables::load_tss,
    },
    structures::{
        gdt::{Descriptor, GlobalDescriptorTable, SegmentSelector},
        tss::TaskStateSegment,
    },
    VirtAddr,
};

pub const DOUBLE_FAULT_IST_INDEX: u16 = 0;
const DOUBLE_FAULT_STACK_SIZE: usize = 4096 * 5;

static mut GDT: GlobalDescriptorTable = GlobalDescriptorTable::new();
static mut TSS: TaskStateSegment = TaskStateSegment::new();
static mut DOUBLE_FAULT_STACK: [u8; DOUBLE_FAULT_STACK_SIZE] = [0; DOUBLE_FAULT_STACK_SIZE];

static mut KERNEL_CODE_SELECTOR: SegmentSelector = SegmentSelector(0);
static mut KERNEL_DATA_SELECTOR: SegmentSelector = SegmentSelector(0);
static mut TSS_SELECTOR: SegmentSelector = SegmentSelector(0);

pub fn init() {
    unsafe {
        kprintln!("[kernel] gdt: Initializing GDT...");

        let stack_start = VirtAddr::from_ptr(DOUBLE_FAULT_STACK.as_ptr());
        let stack_end = stack_start + DOUBLE_FAULT_STACK_SIZE;
        TSS.interrupt_stack_table[DOUBLE_FAULT_IST_INDEX as usize] = stack_end;

        KERNEL_CODE_SELECTOR = GDT.add_entry(Descriptor::kernel_code_segment());
        KERNEL_DATA_SELECTOR = GDT.add_entry(Descriptor::kernel_data_segment());
        TSS_SELECTOR = GDT.add_entry(Descriptor::tss_segment(&TSS));

        GDT.load();

        CS::set_reg(KERNEL_CODE_SELECTOR);
        DS::set_reg(KERNEL_DATA_SELECTOR);
        ES::set_reg(KERNEL_DATA_SELECTOR);
        FS::set_reg(KERNEL_DATA_SELECTOR);
        GS::set_reg(KERNEL_DATA_SELECTOR);
        SS::set_reg(KERNEL_DATA_SELECTOR);
        load_tss(TSS_SELECTOR);

        kprintln!("[kernel] gdt: GDT and TSS loaded.");
    }
}
