//! Keyboard IRQ handler.

use core::sync::atomic::{AtomicU32, Ordering};

use aetheros_common::ipc::keyboard_ipc::KeyEvent;
use x86_64::instructions::port::Port;
use x86_64::structures::idt::InterruptStackFrame;

use crate::{
    ipc,
    kprintln,
    interrupts::{pic, IRQ_KEYBOARD},
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
        unsafe {
            // SAFETY: We are in IRQ1 context and no userspace ACK will follow.
            pic::end_of_interrupt(IRQ_KEYBOARD);
        }
        return;
    }

    let key_event = KeyEvent::new(scancode, None);
    let payload = match postcard::to_allocvec(&key_event) {
        Ok(payload) => payload,
        Err(err) => {
            kprintln!(
                "[kernel] keyboard: failed to serialize KeyEvent for scancode 0x{:02x}: {:?}",
                scancode,
                err
            );
            unsafe {
                // SAFETY: Event encoding failed, so no userspace ACK will follow.
                pic::end_of_interrupt(IRQ_KEYBOARD);
            }
            return;
        }
    };
    if let Err(err) = ipc::mailbox::inject_hardware_event(channel_id, 1, &payload) {
        kprintln!(
            "[kernel] keyboard: failed to route scancode 0x{:02x} to channel {}: {}",
            scancode,
            channel_id,
            err
        );
        unsafe {
            // SAFETY: Delivery failed, so defer no further and ACK in-kernel.
            pic::end_of_interrupt(IRQ_KEYBOARD);
        }
    }

    // NOTE: Hardware EOI for IRQ1 is delegated to the keyboard V-Node via SYS_IRQ_ACK.
}
