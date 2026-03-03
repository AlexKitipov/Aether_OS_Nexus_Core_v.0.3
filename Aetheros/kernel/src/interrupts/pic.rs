//! Intel 8259 PIC support.

use x86_64::instructions::port::Port;

pub const PIC_1_OFFSET: u8 = 32;
pub const PIC_2_OFFSET: u8 = PIC_1_OFFSET + 8;

pub const IRQ_TIMER: u8 = 0;
pub const IRQ_KEYBOARD: u8 = 1;

const PIC_1_CMD: u16 = 0x20;
const PIC_1_DATA: u16 = 0x21;
const PIC_2_CMD: u16 = 0xA0;
const PIC_2_DATA: u16 = 0xA1;

const PIC_EOI: u8 = 0x20;
const ICW1_INIT: u8 = 0x10;
const ICW1_ICW4: u8 = 0x01;
const ICW4_8086: u8 = 0x01;

#[inline]
pub const fn timer_vector() -> u8 {
    PIC_1_OFFSET + IRQ_TIMER
}

#[inline]
pub const fn keyboard_vector() -> u8 {
    PIC_1_OFFSET + IRQ_KEYBOARD
}

#[inline]
unsafe fn io_wait() {
    let mut wait_port: Port<u8> = Port::new(0x80);
    wait_port.write(0);
}

/// Remaps and initializes master/slave PIC.
pub unsafe fn init() {
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

/// Masks (disables) a PIC IRQ line.
pub unsafe fn mask_irq(irq: u8) {
    let (port, bit) = if irq < 8 {
        (PIC_1_DATA, irq)
    } else {
        (PIC_2_DATA, irq - 8)
    };

    let mut data: Port<u8> = Port::new(port);
    let current = data.read();
    data.write(current | (1 << bit));
}

/// Unmasks (enables) a PIC IRQ line.
pub unsafe fn unmask_irq(irq: u8) {
    let (port, bit) = if irq < 8 {
        (PIC_1_DATA, irq)
    } else {
        (PIC_2_DATA, irq - 8)
    };

    let mut data: Port<u8> = Port::new(port);
    let current = data.read();
    data.write(current & !(1 << bit));
}

/// Sends End-of-Interrupt signal for an IRQ.
pub unsafe fn notify_end_of_interrupt(irq: u8) {
    let mut cmd1: Port<u8> = Port::new(PIC_1_CMD);
    let mut cmd2: Port<u8> = Port::new(PIC_2_CMD);

    if irq >= 8 {
        cmd2.write(PIC_EOI);
    }

    cmd1.write(PIC_EOI);
}
