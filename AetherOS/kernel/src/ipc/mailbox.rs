
extern crate alloc;

use alloc::collections::VecDeque;
use alloc::collections::BTreeMap;
use alloc::sync::Arc;
use alloc::vec;
use alloc::vec::Vec;
use aetheros_common::channel::id::ChannelId;
use spin::Mutex;
use conquer_once::spin::OnceCell;
use crate::usercopy::{copy_from_user, copy_to_user};

const MAX_MESSAGE_SIZE: usize = 4096; // Maximum size of an IPC message

// A channel represents an endpoint for IPC communication.
// It holds a queue of messages and can be owned by multiple V-Nodes (Weak for clients, Arc for server)
pub struct Channel {
    id: ChannelId,
    message_queue: Mutex<VecDeque<Message>>,
}

pub struct Message {
    pub sender: u32,
    pub data: Vec<u8>,
}

impl Channel {
    fn new(id: ChannelId) -> Self {
        Channel {
            id,
            message_queue: Mutex::new(VecDeque::new()),
        }
    }

    pub fn send(&self, sender: u32, message: &[u8]) -> Result<(), &'static str> {
        if message.len() > MAX_MESSAGE_SIZE {
            return Err("Message too large");
        }
        let mut queue = self.message_queue.lock();
        queue.push_back(Message {
            sender,
            data: message.to_vec(),
        });
        Ok(())
    }

    pub fn recv(&self) -> Option<Message> {
        let mut queue = self.message_queue.lock();
        queue.pop_front()
    }

    pub fn peek(&self) -> bool {
        !self.message_queue.lock().is_empty()
    }
}

// The Mailbox manages all active IPC channels.
// It's a global singleton protected by a spinlock.
pub struct Mailbox {
    next_channel_id: Mutex<ChannelId>,
    channels: Mutex<Vec<Arc<Channel>>>,
    waiters: Mutex<BTreeMap<ChannelId, Vec<u64>>>,
}

impl Mailbox {
    pub const fn new() -> Self {
        Mailbox {
            next_channel_id: Mutex::new(1),
            channels: Mutex::new(Vec::new()),
            waiters: Mutex::new(BTreeMap::new()),
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



    /// Creates a channel with a fixed id if absent, returning the existing id otherwise.
    pub fn create_or_get_channel(&self, id: ChannelId) -> ChannelId {
        if self.get_channel(id).is_some() {
            return id;
        }

        {
            let mut next_id = self.next_channel_id.lock();
            if id >= *next_id {
                *next_id = id.saturating_add(1);
            }
        }

        let channel = Arc::new(Channel::new(id));
        self.channels.lock().push(channel);
        id
    }

    // Get a channel by its ID. Returns an Arc to the channel if found.
    pub fn get_channel(&self, id: ChannelId) -> Option<Arc<Channel>> {
        self.channels.lock().iter().find(|c| c.id == id).cloned()
    }

    fn register_waiter(&self, channel_id: ChannelId, task_id: u64) {
        let mut waiters = self.waiters.lock();
        let channel_waiters = waiters.entry(channel_id).or_default();
        if !channel_waiters.contains(&task_id) {
            channel_waiters.push(task_id);
        }
    }

    fn wake_one_waiter(&self, channel_id: ChannelId) {
        let waiter = {
            let mut waiters = self.waiters.lock();
            let Some(channel_waiters) = waiters.get_mut(&channel_id) else {
                return;
            };
            let task = if channel_waiters.is_empty() {
                None
            } else {
                Some(channel_waiters.remove(0))
            };
            if channel_waiters.is_empty() {
                waiters.remove(&channel_id);
            }
            task
        };

        if let Some(task_id) = waiter {
            crate::task::scheduler::unblock_task(task_id);
        }
    }
}

static MAILBOX: OnceCell<Mailbox> = OnceCell::uninit();

pub fn init() {
    MAILBOX.init_once(|| Mailbox::new());
}

// --- Public API for IPC syscalls ---

pub fn create_channel() -> ChannelId {
    MAILBOX.get().expect("Mailbox not initialized").create_channel()
}

pub fn send(channel_id: ChannelId, sender: u32, message: &[u8]) -> Result<(), &'static str> {
    let mailbox = MAILBOX.get().expect("Mailbox not initialized");
    if let Some(channel) = mailbox.get_channel(channel_id) {
        channel.send(sender, message)?;
        mailbox.wake_one_waiter(channel_id);
        Ok(())
    } else {
        Err("Channel not found")
    }
}

pub fn recv(channel_id: ChannelId) -> Option<Message> {
    MAILBOX
        .get()
        .expect("Mailbox not initialized")
        .get_channel(channel_id)
        .and_then(|channel| channel.recv())
}

pub fn peek(channel_id: ChannelId) -> bool {
    MAILBOX
        .get()
        .expect("Mailbox not initialized")
        .get_channel(channel_id)
        .is_some_and(|channel| channel.peek())
}

pub fn send_message(channel_id: ChannelId, message_ptr: *const u8, message_len: usize) -> Result<(), &'static str> {
    if !crate::caps::Capability::IpcManage.check_current() {
        return Err("Permission denied: No IpcManage capability");
    }

    if message_len > MAX_MESSAGE_SIZE {
        return Err("Message too large");
    }

    let mailbox = MAILBOX.get().expect("Mailbox not initialized");
    if let Some(channel) = mailbox.get_channel(channel_id) {
        let mut message = vec![0u8; message_len];
        copy_from_user(&mut message, message_ptr)?;
        let sender = crate::task::scheduler::get_current_task_id() as u32;
        channel.send(sender, &message)?;
        mailbox.wake_one_waiter(channel_id);
        Ok(())
    } else {
        Err("Channel not found")
    }
}

/// Injects a kernel-originated hardware event into a mailbox channel.
///
/// This path bypasses userspace buffer copying and capability checks because the
/// event data is already owned by the kernel and arrives from an IRQ context.
pub fn inject_hardware_event(channel_id: ChannelId, irq: u8, payload: &[u8]) -> Result<(), &'static str> {
    send(channel_id, irq as u32, payload)
}

pub fn recv_message(channel_id: ChannelId, buffer_ptr: *mut u8, buffer_len: usize, blocking: bool) -> Result<usize, &'static str> {
    if !crate::caps::Capability::IpcManage.check_current() {
        return Err("Permission denied: No IpcManage capability");
    }

    let mailbox = MAILBOX.get().expect("Mailbox not initialized");
    if let Some(channel) = mailbox.get_channel(channel_id) {
        loop {
            if let Some(message) = channel.recv() {
                if message.data.len() > buffer_len {
                    return Err("Buffer too small");
                }
                copy_to_user(buffer_ptr, &message.data)?;
                return Ok(message.data.len());
            } else if !blocking {
                return Ok(0); // No message, non-blocking
            }

            let current_id = crate::task::scheduler::get_current_task_id();
            mailbox.register_waiter(channel_id, current_id);
            crate::task::scheduler::block_current_task();
        }
    } else {
        Err("Channel not found")
    }
}


pub fn ensure_channel(channel_id: ChannelId) -> ChannelId {
    MAILBOX
        .get()
        .expect("Mailbox not initialized")
        .create_or_get_channel(channel_id)
}
