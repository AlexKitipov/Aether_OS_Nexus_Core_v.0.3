//! Keyboard IRQ handler.

use x86_64::instructions::port::Port;
use x86_64::structures::idt::InterruptStackFrame;

use crate::interrupts::pic;

pub extern "x86-interrupt" fn handler(_stack_frame: InterruptStackFrame) {
    let mut data_port: Port<u8> = Port::new(0x60);
    let _scancode = unsafe { data_port.read() };
    // NOTE: Avoid taking any contended locks (e.g., serial console logging) in IRQ context.
    // Logging from here can deadlock if IRQ1 preempts code that already holds the lock.

    unsafe {
        pic::notify_end_of_interrupt(pic::IRQ_KEYBOARD);
    }
}
