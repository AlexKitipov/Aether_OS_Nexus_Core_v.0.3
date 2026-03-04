// kernel/src/ipc.rs

#![allow(dead_code)] // Allow dead code for now as not all functions might be used immediately

extern crate alloc; // Still needed by mailbox, but declared in lib.rs

use crate::{kprintln}; // kprintln still needed for init func

pub mod mailbox; // Declare the new mailbox module

// Re-export public items from the mailbox module to maintain the ipc facade
pub use mailbox::{ChannelId, Message, send as kernel_send, recv as kernel_recv, peek as kernel_peek};

/// Initializes the IPC module.
pub fn init() {
    kprintln!("[kernel] ipc: Initialized.");
    // No specific initialization for mailbox itself as its statics are lazy initialized or used directly.
}
