//! Keyboard IRQ handler.

use x86_64::instructions::port::Port;
use x86_64::structures::idt::InterruptStackFrame;

use crate::{
    interrupts::{pic, IRQ_KEYBOARD},
    kprintln,
};

const KEYBOARD_DATA_PORT: u16 = 0x60;

pub extern "x86-interrupt" fn handler(_stack_frame: InterruptStackFrame) {
    let mut data_port: Port<u8> = Port::new(KEYBOARD_DATA_PORT);
    // SAFETY: Reading from port 0x60 is the required way to consume keyboard IRQ data
    // on the legacy PS/2 controller in this execution environment.
    let scancode = unsafe { data_port.read() };

    kprintln!("[kernel] keyboard: scancode=0x{:02x}", scancode);

    // SAFETY: This IRQ was raised by the keyboard line, so notifying the PIC with the
    // corresponding IRQ number is required to re-enable subsequent interrupts.
    unsafe { pic::end_of_interrupt(IRQ_KEYBOARD) };
}
