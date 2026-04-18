// kernel/src/arch/x86_64/gdt.rs

#![allow(dead_code)]

use core::ptr::addr_of;

use spin::Once;
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GdtState {
    pub initialized: bool,
    pub double_fault_ist_index: u16,
    pub double_fault_stack_start: u64,
    pub double_fault_stack_end: u64,
    pub code_selector: u16,
    pub data_selector: u16,
    pub user_code_selector: u16,
    pub user_data_selector: u16,
    pub tss_selector: u16,
}

#[repr(align(16))]
struct DoubleFaultStack([u8; DOUBLE_FAULT_STACK_SIZE]);

// Keep IST memory 16-byte aligned. x86_64 interrupt/trap entry itself is not
// required to maintain SysV alignment, but handlers and called Rust code may
// still rely on aligned stack accesses.
static DOUBLE_FAULT_STACK: DoubleFaultStack = DoubleFaultStack([0; DOUBLE_FAULT_STACK_SIZE]);
static TSS: Once<TaskStateSegment> = Once::new();
static GDT: Once<(GlobalDescriptorTable, Selectors)> = Once::new();

fn double_fault_stack_range() -> (VirtAddr, VirtAddr) {
    let stack_start = VirtAddr::from_ptr(addr_of!(DOUBLE_FAULT_STACK.0));
    let stack_end = stack_start + DOUBLE_FAULT_STACK_SIZE;
    (stack_start, stack_end)
}

fn ensure_gdt() -> &'static (GlobalDescriptorTable, Selectors) {
    GDT.call_once(|| {
        let tss = TSS.call_once(|| {
            let mut tss = TaskStateSegment::new();
            // SAFETY: `addr_of!` avoids creating references to mutable memory.
            // We only use the raw address of the dedicated static stack buffer.
            // The stack range is a valid, 16-byte-aligned static allocation and
            // IST uses the end address because stacks grow downward on x86_64.
            let (_, stack_end) = double_fault_stack_range();
            tss.interrupt_stack_table[DOUBLE_FAULT_IST_INDEX as usize] = stack_end;
            tss
        });

        let mut gdt = GlobalDescriptorTable::new();
        let code_selector = gdt.add_entry(Descriptor::kernel_code_segment());
        let data_selector = gdt.add_entry(Descriptor::kernel_data_segment());
        let user_data_selector = gdt.add_entry(Descriptor::user_data_segment());
        let user_code_selector = gdt.add_entry(Descriptor::user_code_segment());
        let tss_selector = gdt.add_entry(Descriptor::tss_segment(tss));

        (
            gdt,
            Selectors {
                code_selector,
                data_selector,
                user_code_selector,
                user_data_selector,
                tss_selector,
            },
        )
    })
}

pub fn init() {
    kprintln!("[kernel] gdt: Initializing GDT and TSS...");

    ensure_gdt();

    let (gdt, selectors) = ensure_gdt();
    // SAFETY: segment register writes and TSS load are privileged CPU state
    // updates and require ring-0 execution during early kernel setup.
    unsafe {
        gdt.load();
        CS::set_reg(selectors.code_selector);
        DS::set_reg(selectors.data_selector);
        load_tss(selectors.tss_selector);
    }

    kprintln!("[kernel] gdt: GDT and TSS loaded (Ring 0 and Ring 3 ready).");
}

pub fn get_selectors() -> &'static Selectors {
    &ensure_gdt().1
}

pub fn state() -> GdtState {
    let (_, selectors) = ensure_gdt();
    let (stack_start, stack_end) = double_fault_stack_range();

    GdtState {
        initialized: true,
        double_fault_ist_index: DOUBLE_FAULT_IST_INDEX,
        double_fault_stack_start: stack_start.as_u64(),
        double_fault_stack_end: stack_end.as_u64(),
        code_selector: selectors.code_selector.0,
        data_selector: selectors.data_selector.0,
        user_code_selector: selectors.user_code_selector.0,
        user_data_selector: selectors.user_data_selector.0,
        tss_selector: selectors.tss_selector.0,
    }
}
