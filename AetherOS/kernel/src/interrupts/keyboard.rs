//! Keyboard IRQ handler.

use x86_64::instructions::port::Port;
use x86_64::structures::idt::InterruptStackFrame;

use crate::{
    ipc,
    kprintln,
};

const KEYBOARD_DATA_PORT: u16 = 0x60;
const KEYBOARD_VNODE_CHANNEL_ID: u32 = 4;

pub extern "x86-interrupt" fn handler(_stack_frame: InterruptStackFrame) {
    let mut data_port: Port<u8> = Port::new(KEYBOARD_DATA_PORT);
    // SAFETY: Reading from port 0x60 is the required way to consume keyboard IRQ data
    // on the legacy PS/2 controller in this execution environment.
    let scancode = unsafe { data_port.read() };

    let payload = [scancode];
    if let Err(err) = ipc::mailbox::send(KEYBOARD_VNODE_CHANNEL_ID, 0, &payload) {
        kprintln!(
            "[kernel] keyboard: failed to route scancode 0x{:02x} to channel {}: {}",
            scancode,
            KEYBOARD_VNODE_CHANNEL_ID,
            err
        );
    }

    // NOTE: Hardware EOI for IRQ1 is delegated to the keyboard V-Node via SYS_IRQ_ACK.
}
