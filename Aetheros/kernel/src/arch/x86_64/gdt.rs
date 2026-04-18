// kernel/src/arch/x86_64/gdt.rs

use x86_64::instructions::segmentation::{Segment, CS, DS, ES, FS, GS, SS};
use x86_64::instructions::tables::load_tss;
use x86_64::structures::gdt::{Descriptor, GlobalDescriptorTable, SegmentSelector};
use x86_64::structures::tss::TaskStateSegment;
use x86_64::VirtAddr;

use crate::kprintln;

/// IST slot used by the double-fault handler.
pub const DOUBLE_FAULT_IST_INDEX: u16 = 0;

const DOUBLE_FAULT_STACK_SIZE: usize = 4096 * 5;

struct Selectors {
    kernel_code: SegmentSelector,
    kernel_data: SegmentSelector,
    user_code: SegmentSelector,
    user_data: SegmentSelector,
    tss: SegmentSelector,
}

struct GdtState {
    _gdt: GlobalDescriptorTable,
    selectors: Selectors,
}

static mut DOUBLE_FAULT_STACK: [u8; DOUBLE_FAULT_STACK_SIZE] = [0; DOUBLE_FAULT_STACK_SIZE];
static mut TSS: Option<TaskStateSegment> = None;
static mut GDT_STATE: Option<GdtState> = None;

pub fn init() {
    // SAFETY: boot-time, single-core initialization path.
    unsafe {
        kprintln!("[kernel] gdt: Initializing GDT/TSS...");

        let mut tss = TaskStateSegment::new();
        let stack_start = VirtAddr::from_ptr(core::ptr::addr_of!(DOUBLE_FAULT_STACK));
        tss.interrupt_stack_table[DOUBLE_FAULT_IST_INDEX as usize] =
            stack_start + DOUBLE_FAULT_STACK_SIZE;
        TSS = Some(tss);

        let mut gdt = GlobalDescriptorTable::new();
        let kernel_code = gdt.add_entry(Descriptor::kernel_code_segment());
        let kernel_data = gdt.add_entry(Descriptor::kernel_data_segment());
        let user_data = gdt.add_entry(Descriptor::user_data_segment());
        let user_code = gdt.add_entry(Descriptor::user_code_segment());
        let tss_selector = gdt.add_entry(Descriptor::tss_segment(TSS.as_ref().unwrap()));

        let selectors = Selectors {
            kernel_code,
            kernel_data,
            user_code,
            user_data,
            tss: tss_selector,
        };

        gdt.load();

        CS::set_reg(selectors.kernel_code);
        DS::set_reg(selectors.kernel_data);
        ES::set_reg(selectors.kernel_data);
        FS::set_reg(selectors.kernel_data);
        GS::set_reg(selectors.kernel_data);
        SS::set_reg(selectors.kernel_data);
        load_tss(selectors.tss);

        GDT_STATE = Some(GdtState {
            _gdt: gdt,
            selectors,
        });

        kprintln!(
            "[kernel] gdt: Loaded (kcode={:?}, kdata={:?}, ucode={:?}, udata={:?}, tss={:?}).",
            GDT_STATE.as_ref().unwrap().selectors.kernel_code,
            GDT_STATE.as_ref().unwrap().selectors.kernel_data,
            GDT_STATE.as_ref().unwrap().selectors.user_code,
            GDT_STATE.as_ref().unwrap().selectors.user_data,
            GDT_STATE.as_ref().unwrap().selectors.tss,
        );
    }
}
