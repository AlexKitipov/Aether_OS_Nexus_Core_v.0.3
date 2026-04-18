#![no_std]

extern crate alloc;

use alloc::collections::VecDeque;
use alloc::sync::Arc;
use alloc::vec::Vec;
use common::channel::id::ChannelId;
use conquer_once::spin::Once;
use spin::Mutex;

const MAX_MESSAGE_SIZE: usize = 4096;
const DEFAULT_CHANNEL_DEPTH: usize = 64;
const DEFAULT_CHANNEL_INFLIGHT_BYTES: usize = 256 * 1024;

#[derive(Debug, Clone)]
pub struct SharedMemoryGrant {
    pub grant_id: u64,
    pub len: usize,
    pub writable: bool,
}

#[derive(Debug, Clone)]
pub enum MessagePayload {
    Inline(Vec<u8>),
    SharedMemory(SharedMemoryGrant),
}

#[derive(Debug, Clone)]
pub struct Message {
    pub sender_task_id: u64,
    pub payload: MessagePayload,
}

impl Message {
    pub fn as_inline(&self) -> Option<&[u8]> {
        match &self.payload {
            MessagePayload::Inline(bytes) => Some(bytes.as_slice()),
            MessagePayload::SharedMemory(_) => None,
        }
    }
}

pub struct Channel {
    id: ChannelId,
    queue: Mutex<VecDeque<Message>>,
    receiver_waiters: Mutex<VecDeque<u64>>,
    max_depth: usize,
    max_inflight_bytes: usize,
    inflight_bytes: Mutex<usize>,
}

impl Channel {
    fn new(id: ChannelId) -> Self {
        Self {
            id,
            queue: Mutex::new(VecDeque::new()),
            receiver_waiters: Mutex::new(VecDeque::new()),
            max_depth: DEFAULT_CHANNEL_DEPTH,
            max_inflight_bytes: DEFAULT_CHANNEL_INFLIGHT_BYTES,
            inflight_bytes: Mutex::new(0),
        }
    }

    fn enqueue(&self, message: Message) -> Result<Option<u64>, &'static str> {
        let inline_len = match &message.payload {
            MessagePayload::Inline(bytes) => {
                if bytes.len() > MAX_MESSAGE_SIZE {
                    return Err("message too large");
                }
                bytes.len()
            }
            MessagePayload::SharedMemory(grant) => grant.len,
        };

        let mut queue = self.queue.lock();
        if queue.len() >= self.max_depth {
            return Err("channel queue full");
        }

        let mut inflight = self.inflight_bytes.lock();
        if *inflight + inline_len > self.max_inflight_bytes {
            return Err("channel inflight byte budget exceeded");
        }

        *inflight += inline_len;
        queue.push_back(message);
        drop(inflight);
        drop(queue);

        Ok(self.receiver_waiters.lock().pop_front())
    }

    fn dequeue(&self) -> Option<Message> {
        let mut queue = self.queue.lock();
        let message = queue.pop_front();
        if let Some(msg) = &message {
            let released = match &msg.payload {
                MessagePayload::Inline(bytes) => bytes.len(),
                MessagePayload::SharedMemory(grant) => grant.len,
            };
            let mut inflight = self.inflight_bytes.lock();
            *inflight = inflight.saturating_sub(released);
        }
        message
    }

    fn has_pending(&self) -> bool {
        !self.queue.lock().is_empty()
    }

    fn register_receiver_waiter(&self, task_id: u64) {
        let mut waiters = self.receiver_waiters.lock();
        if !waiters.iter().any(|&waiting| waiting == task_id) {
            waiters.push_back(task_id);
        }
    }
}

pub struct Mailbox {
    next_channel_id: Mutex<ChannelId>,
    channels: Mutex<Vec<Arc<Channel>>>,
}

impl Mailbox {
    pub const fn new() -> Self {
        Self {
            next_channel_id: Mutex::new(1),
            channels: Mutex::new(Vec::new()),
        }
    }

    pub fn create_channel(&self) -> ChannelId {
        let mut next_id = self.next_channel_id.lock();
        let new_id = *next_id;
        *next_id += 1;

        self.channels.lock().push(Arc::new(Channel::new(new_id)));
        new_id
    }

    pub fn get_channel(&self, id: ChannelId) -> Option<Arc<Channel>> {
        self.channels.lock().iter().find(|c| c.id == id).cloned()
    }
}

static MAILBOX: Once<Mailbox> = Once::new();

pub fn init() {
    MAILBOX.call_once(Mailbox::new);
}

fn mailbox() -> &'static Mailbox {
    MAILBOX.get().expect("Mailbox not initialized")
}

pub fn create_channel() -> ChannelId {
    mailbox().create_channel()
}

pub fn send(channel_id: ChannelId, sender_task_id: u64, message: &[u8]) -> Result<(), &'static str> {
    let channel = mailbox().get_channel(channel_id).ok_or("channel not found")?;
    let wake_task = channel.enqueue(Message {
        sender_task_id,
        payload: MessagePayload::Inline(message.to_vec()),
    })?;

    if let Some(task_id) = wake_task {
        crate::task::unblock_task_on_channel(task_id);
    }

    Ok(())
}

pub fn send_shared_memory(
    channel_id: ChannelId,
    sender_task_id: u64,
    grant_id: u64,
    len: usize,
    writable: bool,
) -> Result<(), &'static str> {
    let channel = mailbox().get_channel(channel_id).ok_or("channel not found")?;
    let wake_task = channel.enqueue(Message {
        sender_task_id,
        payload: MessagePayload::SharedMemory(SharedMemoryGrant {
            grant_id,
            len,
            writable,
        }),
    })?;

    if let Some(task_id) = wake_task {
        crate::task::unblock_task_on_channel(task_id);
    }

    Ok(())
}

pub fn recv(channel_id: ChannelId) -> Option<Message> {
    mailbox().get_channel(channel_id).and_then(|channel| channel.dequeue())
}

pub fn peek(channel_id: ChannelId) -> bool {
    mailbox()
        .get_channel(channel_id)
        .map(|channel| channel.has_pending())
        .unwrap_or(false)
}

pub fn register_receiver_waiter(channel_id: ChannelId, task_id: u64) -> Result<(), &'static str> {
    let channel = mailbox().get_channel(channel_id).ok_or("channel not found")?;
    channel.register_receiver_waiter(task_id);
    Ok(())
}
