//! Intel 8259 PIC support.

use x86_64::instructions::port::Port;

pub const PIC_1_OFFSET: u8 = 32;
pub const PIC_2_OFFSET: u8 = PIC_1_OFFSET + 8;

pub const PIC_1_CMD: u16 = 0x20;
pub const PIC_1_DATA: u16 = 0x21;
pub const PIC_2_CMD: u16 = 0xA0;
pub const PIC_2_DATA: u16 = 0xA1;

const PIC_EOI: u8 = 0x20;
const ICW1_INIT: u8 = 0x10;
const ICW1_ICW4: u8 = 0x01;
const ICW4_8086: u8 = 0x01;

#[inline]
unsafe fn io_wait() {
    let mut wait_port: Port<u8> = Port::new(0x80);
    wait_port.write(0);
}

/// Remaps and initializes master/slave PIC.
pub unsafe fn remap() {
    let mut cmd1: Port<u8> = Port::new(PIC_1_CMD);
    let mut data1: Port<u8> = Port::new(PIC_1_DATA);
    let mut cmd2: Port<u8> = Port::new(PIC_2_CMD);
    let mut data2: Port<u8> = Port::new(PIC_2_DATA);

    // Save current masks.
    let mask1 = data1.read();
    let mask2 = data2.read();

    cmd1.write(ICW1_INIT | ICW1_ICW4);
    io_wait();
    cmd2.write(ICW1_INIT | ICW1_ICW4);
    io_wait();

    data1.write(PIC_1_OFFSET);
    io_wait();
    data2.write(PIC_2_OFFSET);
    io_wait();

    // Tell master PIC about slave PIC at IRQ2.
    data1.write(4);
    io_wait();
    // Tell slave PIC its cascade identity.
    data2.write(2);
    io_wait();

    data1.write(ICW4_8086);
    io_wait();
    data2.write(ICW4_8086);
    io_wait();

    // Restore masks.
    data1.write(mask1);
    data2.write(mask2);
}

/// Sends End-of-Interrupt signal for an IRQ.
pub unsafe fn end_of_interrupt(irq: u8) {
    let mut cmd1: Port<u8> = Port::new(PIC_1_CMD);
    let mut cmd2: Port<u8> = Port::new(PIC_2_CMD);

    if irq >= 8 {
        cmd2.write(PIC_EOI);
    }

    cmd1.write(PIC_EOI);
}
