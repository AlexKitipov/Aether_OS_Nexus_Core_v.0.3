//! Keyboard IRQ handler.

use core::sync::atomic::{AtomicU32, Ordering};

use x86_64::instructions::port::Port;
use x86_64::structures::idt::InterruptStackFrame;

use crate::{
    ipc,
    kprintln,
};

const KEYBOARD_DATA_PORT: u16 = 0x60;
const UNREGISTERED_CHANNEL: u32 = 0;

static KEYBOARD_IRQ_CHANNEL_ID: AtomicU32 = AtomicU32::new(UNREGISTERED_CHANNEL);

/// Registers the IPC channel that should receive keyboard scancode events.
pub fn register_channel(channel_id: u32) {
    KEYBOARD_IRQ_CHANNEL_ID.store(channel_id, Ordering::Release);
    kprintln!(
        "[kernel] keyboard: routing IRQ1 scancodes to IPC channel {}.",
        channel_id
    );
}

pub extern "x86-interrupt" fn handler(_stack_frame: InterruptStackFrame) {
    let mut data_port: Port<u8> = Port::new(KEYBOARD_DATA_PORT);
    // SAFETY: Reading from port 0x60 is the required way to consume keyboard IRQ data
    // on the legacy PS/2 controller in this execution environment.
    let scancode = unsafe { data_port.read() };

    let channel_id = KEYBOARD_IRQ_CHANNEL_ID.load(Ordering::Acquire);
    if channel_id == UNREGISTERED_CHANNEL {
        kprintln!(
            "[kernel] keyboard: dropped scancode 0x{:02x}; no registered keyboard V-Node.",
            scancode
        );
        return;
    }

    let payload = [scancode];
    if let Err(err) = ipc::mailbox::inject_hardware_event(channel_id, 1, &payload) {
        kprintln!(
            "[kernel] keyboard: failed to route scancode 0x{:02x} to channel {}: {}",
            scancode,
            channel_id,
            err
        );
    }

    // NOTE: Hardware EOI for IRQ1 is delegated to the keyboard V-Node via SYS_IRQ_ACK.
}
