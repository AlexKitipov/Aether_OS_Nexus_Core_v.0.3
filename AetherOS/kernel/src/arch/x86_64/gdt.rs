// kernel/src/arch/x86_64/gdt.rs

#![allow(dead_code)]

use core::ptr::addr_of;

use x86_64::instructions::segmentation::{CS, DS, Segment};
use x86_64::instructions::tables::load_tss;
use x86_64::structures::gdt::{Descriptor, GlobalDescriptorTable, SegmentSelector};
use x86_64::structures::tss::TaskStateSegment;
use x86_64::VirtAddr;

use crate::kprintln;

pub const DOUBLE_FAULT_IST_INDEX: u16 = 0;

const DOUBLE_FAULT_STACK_SIZE: usize = 4096 * 5;

pub struct Selectors {
    pub code_selector: SegmentSelector,
    pub data_selector: SegmentSelector,
    pub user_code_selector: SegmentSelector,
    pub user_data_selector: SegmentSelector,
    pub tss_selector: SegmentSelector,
}

static mut DOUBLE_FAULT_STACK: [u8; DOUBLE_FAULT_STACK_SIZE] = [0; DOUBLE_FAULT_STACK_SIZE];
static mut TSS: Option<TaskStateSegment> = None;
static mut GDT: Option<(GlobalDescriptorTable, Selectors)> = None;

fn ensure_gdt() {
    unsafe {
        if GDT.is_some() {
            return;
        }

        let mut tss = TaskStateSegment::new();
        let stack_start = VirtAddr::from_ptr(addr_of!(DOUBLE_FAULT_STACK));
        let stack_end = stack_start + DOUBLE_FAULT_STACK_SIZE;
        tss.interrupt_stack_table[DOUBLE_FAULT_IST_INDEX as usize] = stack_end;
        TSS = Some(tss);

        let mut gdt = GlobalDescriptorTable::new();
        let code_selector = gdt.add_entry(Descriptor::kernel_code_segment());
        let data_selector = gdt.add_entry(Descriptor::kernel_data_segment());
        let user_data_selector = gdt.add_entry(Descriptor::user_data_segment());
        let user_code_selector = gdt.add_entry(Descriptor::user_code_segment());
        let tss_selector =
            gdt.add_entry(Descriptor::tss_segment(TSS.as_ref().expect("TSS initialized")));

        GDT = Some((
            gdt,
            Selectors {
                code_selector,
                data_selector,
                user_code_selector,
                user_data_selector,
                tss_selector,
            },
        ));
    }
}

pub fn init() {
    kprintln!("[kernel] gdt: Initializing GDT and TSS...");

    ensure_gdt();

    unsafe {
        let (gdt, selectors) = GDT.as_ref().expect("GDT initialized");
        gdt.load();
        CS::set_reg(selectors.code_selector);
        DS::set_reg(selectors.data_selector);
        load_tss(selectors.tss_selector);
    }

    kprintln!("[kernel] gdt: GDT and TSS loaded (Ring 0 and Ring 3 ready).");
}

pub fn get_selectors() -> &'static Selectors {
    ensure_gdt();
    unsafe { &GDT.as_ref().expect("GDT initialized").1 }
}
