//! Keyboard IRQ handler.

use x86_64::instructions::port::Port;
use x86_64::structures::idt::InterruptStackFrame;

use crate::{interrupts::pic, kprintln};

pub extern "x86-interrupt" fn handler(_stack_frame: InterruptStackFrame) {
    let mut data_port: Port<u8> = Port::new(0x60);
    let scancode = unsafe { data_port.read() };

    kprintln!("[kernel] keyboard: scancode=0x{:02x}", scancode);

    unsafe {
        pic::notify_end_of_interrupt(pic::IRQ_KEYBOARD);
    }
}
