#![no_std] // Don't link the Rust standard library

extern crate alloc;

use alloc::collections::VecDeque;
use alloc::sync::{Arc, Weak};
use alloc::vec::Vec;
use common::channel::id::ChannelId;
use spin::Mutex;
use conquer_once::spin::Once;

const MAX_MESSAGE_SIZE: usize = 4096; // Maximum size of an IPC message

// A channel represents an endpoint for IPC communication.
// It holds a queue of messages and can be owned by multiple V-Nodes (Weak for clients, Arc for server)
pub struct Channel {
    id: ChannelId,
    message_queue: Mutex<VecDeque<Vec<u8>>>,
}

impl Channel {
    fn new(id: ChannelId) -> Self {
        Channel {
            id,
            message_queue: Mutex::new(VecDeque::new()),
        }
    }

    pub fn send(&self, message: &[u8]) -> Result<(), &'static str> {
        if message.len() > MAX_MESSAGE_SIZE {
            return Err("Message too large");
        }
        let mut queue = self.message_queue.lock();
        queue.push_back(message.to_vec());
        Ok(())
    }

    pub fn recv(&self) -> Option<Vec<u8>> {
        let mut queue = self.message_queue.lock();
        queue.pop_front()
    }
}

// The Mailbox manages all active IPC channels.
// It's a global singleton protected by a spinlock.
pub struct Mailbox {
    next_channel_id: Mutex<ChannelId>,
    channels: Mutex<Vec<Arc<Channel>>>,
}

impl Mailbox {
    pub const fn new() -> Self {
        Mailbox {
            next_channel_id: Mutex::new(1),
            channels: Mutex::new(Vec::new()),
        }
    }

    // Create a new channel and return its ID.
    pub fn create_channel(&self) -> ChannelId {
        let mut next_id = self.next_channel_id.lock();
        let new_id = *next_id;
        *next_id += 1;

        let channel = Arc::new(Channel::new(new_id));
        self.channels.lock().push(channel);

        new_id
    }

    // Get a channel by its ID. Returns an Arc to the channel if found.
    pub fn get_channel(&self, id: ChannelId) -> Option<Arc<Channel>> {
        self.channels.lock().iter().find(|c| c.id == id).cloned()
    }
}

static MAILBOX: Once<Mailbox> = Once::new();

pub fn init() {
    MAILBOX.call_once(|| Mailbox::new());
}

// --- Public API for IPC syscalls ---

pub fn create_channel() -> ChannelId {
    MAILBOX.get().expect("Mailbox not initialized").create_channel()
}

pub fn send_message(channel_id: ChannelId, message_ptr: *const u8, message_len: usize) -> Result<(), &'static str> {
    let mailbox = MAILBOX.get().expect("Mailbox not initialized");
    if let Some(channel) = mailbox.get_channel(channel_id) {
        let message = unsafe { core::slice::from_raw_parts(message_ptr, message_len) };
        channel.send(message)
    } else {
        Err("Channel not found")
    }
}

pub fn recv_message(channel_id: ChannelId, buffer_ptr: *mut u8, buffer_len: usize, blocking: bool) -> Result<usize, &'static str> {
    let mailbox = MAILBOX.get().expect("Mailbox not initialized");
    if let Some(channel) = mailbox.get_channel(channel_id) {
        loop {
            if let Some(message) = channel.recv() {
                if message.len() > buffer_len {
                    return Err("Buffer too small");
                }
                unsafe { core::ptr::copy_nonoverlapping(message.as_ptr(), buffer_ptr, message.len()) };
                return Ok(message.len());
            } else if !blocking {
                return Ok(0); // No message, non-blocking
            }
            // If blocking and no message, yield CPU (actual kernel would involve sleeping task)
            unsafe { common::syscall::syscall3(common::syscall::SYS_TIME, 0, 0, 0); }
        }
    } else {
        Err("Channel not found")
    }
}

