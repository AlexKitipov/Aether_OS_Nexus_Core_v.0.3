// kernel/src/ipc.rs

#![allow(dead_code)]

use crate::kprintln;

pub mod mailbox;

pub use mailbox::{
    ChannelId,
    Message,
    MessagePayload,
    SharedMemoryGrant,
    create_channel,
    peek as kernel_peek,
    recv as kernel_recv,
    register_receiver_waiter as kernel_register_receiver_waiter,
    send as kernel_send,
    send_shared_memory as kernel_send_shared_memory,
};

/// Initializes the IPC module.
pub fn init() {
    mailbox::init();
    kprintln!("[kernel] ipc: Initialized.");
}
