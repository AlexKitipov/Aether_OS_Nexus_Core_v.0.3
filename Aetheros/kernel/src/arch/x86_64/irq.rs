// kernel/src/arch/x86_64/irq.rs

#![allow(dead_code)]

use crate::{arch::x86_64::idt, ipc, kprintln};
use alloc::collections::BTreeMap;
use spin::Mutex;
use x86_64::instructions::port::Port;

const PIC_1_COMMAND: u16 = 0x20;
const PIC_1_DATA: u16 = 0x21;
const PIC_2_COMMAND: u16 = 0xA0;
const PIC_2_DATA: u16 = 0xA1;
const PIC_EOI: u8 = 0x20;

const ICW1_INIT: u8 = 0x10;
const ICW1_ICW4: u8 = 0x01;
const ICW4_8086: u8 = 0x01;

static IRQ_TO_CHANNEL_MAP: Mutex<BTreeMap<u8, ipc::ChannelId>> = Mutex::new(BTreeMap::new());

pub unsafe fn init_pic() {
    kprintln!("[kernel] irq: Initializing PIC...");

    let mut pic1_cmd = Port::<u8>::new(PIC_1_COMMAND);
    let mut pic1_data = Port::<u8>::new(PIC_1_DATA);
    let mut pic2_cmd = Port::<u8>::new(PIC_2_COMMAND);
    let mut pic2_data = Port::<u8>::new(PIC_2_DATA);

    let pic1_mask = pic1_data.read();
    let pic2_mask = pic2_data.read();

    pic1_cmd.write(ICW1_INIT | ICW1_ICW4);
    pic2_cmd.write(ICW1_INIT | ICW1_ICW4);

    pic1_data.write(idt::PIC_1_OFFSET);
    pic2_data.write(idt::PIC_2_OFFSET);

    pic1_data.write(4);
    pic2_data.write(2);

    pic1_data.write(ICW4_8086);
    pic2_data.write(ICW4_8086);

    // Keep IRQ0 (timer) and IRQ1 (keyboard) enabled by default.
    pic1_data.write(pic1_mask & !0b0000_0011);
    pic2_data.write(pic2_mask);

    kprintln!("[kernel] irq: PIC initialized and remapped.");
}

pub fn register_irq_handler(irq_number: u8, channel_id: ipc::ChannelId) {
    let mut map = IRQ_TO_CHANNEL_MAP.lock();
    map.insert(irq_number, channel_id);
    kprintln!(
        "[kernel] irq: Registered IRQ {} to IPC channel {}.",
        irq_number,
        channel_id
    );
}

pub fn acknowledge_irq(irq_number: u8) {
    unsafe {
        if irq_number >= 8 {
            Port::<u8>::new(PIC_2_COMMAND).write(PIC_EOI);
        }
        Port::<u8>::new(PIC_1_COMMAND).write(PIC_EOI);
    }
}

pub fn handle_irq(irq_number: u8) {
    let channel_id = {
        let map = IRQ_TO_CHANNEL_MAP.lock();
        map.get(&irq_number).cloned()
    };

    if let Some(id) = channel_id {
        let irq_msg_data = alloc::vec![irq_number];
        let _ = ipc::kernel_send(id, 0, &irq_msg_data);
    } else {
        kprintln!("[kernel] irq: Unhandled IRQ {}.", irq_number);
    }

    acknowledge_irq(irq_number);
}
