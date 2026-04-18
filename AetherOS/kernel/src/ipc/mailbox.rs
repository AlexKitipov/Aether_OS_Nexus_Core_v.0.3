extern crate alloc;

use alloc::collections::VecDeque;
use alloc::sync::Arc;
use alloc::vec;
use alloc::vec::Vec;
use aetheros_common::channel::id::ChannelId;
use conquer_once::spin::OnceCell;
use spin::Mutex;

use crate::usercopy::{copy_from_user, copy_to_user};

const MAX_INLINE_MESSAGE_SIZE: usize = 4096;
const DEFAULT_MAX_DEPTH: usize = 64;
const DEFAULT_MAX_INFLIGHT_BYTES: usize = 256 * 1024;

#[derive(Clone)]
pub struct SharedMemoryGrant {
    owner_task_id: u64,
    data: Arc<[u8]>,
}

impl SharedMemoryGrant {
    pub fn new(owner_task_id: u64, data: Vec<u8>) -> Self {
        Self {
            owner_task_id,
            data: data.into(),
        }
    }

    pub fn owner_task_id(&self) -> u64 {
        self.owner_task_id
    }

    pub fn len(&self) -> usize {
        self.data.len()
    }

    pub fn is_empty(&self) -> bool {
        self.data.is_empty()
    }

    pub fn as_slice(&self) -> &[u8] {
        &self.data
    }
}

#[derive(Clone)]
pub enum MessagePayload {
    Inline(Vec<u8>),
    SharedMemory(SharedMemoryGrant),
}

#[derive(Clone)]
pub struct Message {
    pub sender: u32,
    pub payload: MessagePayload,
}

impl Message {
    fn payload_len(&self) -> usize {
        match &self.payload {
            MessagePayload::Inline(data) => data.len(),
            MessagePayload::SharedMemory(grant) => grant.len(),
        }
    }
}

pub struct Channel {
    id: ChannelId,
    message_queue: Mutex<VecDeque<Message>>,
    receiver_waiters: Mutex<VecDeque<u64>>,
    inflight_bytes: Mutex<usize>,
    max_depth: usize,
    max_inflight_bytes: usize,
}

impl Channel {
    fn new(id: ChannelId, max_depth: usize, max_inflight_bytes: usize) -> Self {
        Self {
            id,
            message_queue: Mutex::new(VecDeque::new()),
            receiver_waiters: Mutex::new(VecDeque::new()),
            inflight_bytes: Mutex::new(0),
            max_depth,
            max_inflight_bytes,
        }
    }

    fn enqueue(&self, sender: u32, payload: MessagePayload) -> Result<(), &'static str> {
        let payload_len = match &payload {
            MessagePayload::Inline(data) => data.len(),
            MessagePayload::SharedMemory(grant) => grant.len(),
        };

        let mut queue = self.message_queue.lock();
        if queue.len() >= self.max_depth {
            return Err("Channel queue full");
        }

        let mut inflight = self.inflight_bytes.lock();
        if payload_len > self.max_inflight_bytes.saturating_sub(*inflight) {
            return Err("Channel inflight byte budget exceeded");
        }

        *inflight += payload_len;
        queue.push_back(Message { sender, payload });
        Ok(())
    }

    pub fn send(&self, sender: u32, message: &[u8]) -> Result<(), &'static str> {
        if message.len() > MAX_INLINE_MESSAGE_SIZE {
            return Err("Inline message too large");
        }
        self.enqueue(sender, MessagePayload::Inline(message.to_vec()))
    }

    pub fn send_shared_memory(
        &self,
        sender: u32,
        grant: SharedMemoryGrant,
    ) -> Result<(), &'static str> {
        self.enqueue(sender, MessagePayload::SharedMemory(grant))
    }

    pub fn recv(&self) -> Option<Message> {
        let message = self.message_queue.lock().pop_front();
        if let Some(message) = &message {
            let payload_len = message.payload_len();
            let mut inflight = self.inflight_bytes.lock();
            *inflight = inflight.saturating_sub(payload_len);
        }
        message
    }

    pub fn peek(&self) -> bool {
        !self.message_queue.lock().is_empty()
    }

    pub fn register_receiver_waiter(&self, task_id: u64) {
        let mut waiters = self.receiver_waiters.lock();
        if waiters.contains(&task_id) {
            return;
        }
        waiters.push_back(task_id);
    }

    pub fn wake_one_receiver(&self) {
        let waiter = self.receiver_waiters.lock().pop_front();
        if let Some(task_id) = waiter {
            crate::task::scheduler::unblock_task(task_id);
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
        self.create_channel_with_limits(DEFAULT_MAX_DEPTH, DEFAULT_MAX_INFLIGHT_BYTES)
    }

    pub fn create_channel_with_limits(&self, max_depth: usize, max_inflight_bytes: usize) -> ChannelId {
        let mut next_id = self.next_channel_id.lock();
        let new_id = *next_id;
        *next_id += 1;

        let channel = Arc::new(Channel::new(new_id, max_depth, max_inflight_bytes));
        self.channels.lock().push(channel);
        new_id
    }

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

        let channel = Arc::new(Channel::new(
            id,
            DEFAULT_MAX_DEPTH,
            DEFAULT_MAX_INFLIGHT_BYTES,
        ));
        self.channels.lock().push(channel);
        id
    }

    pub fn get_channel(&self, id: ChannelId) -> Option<Arc<Channel>> {
        self.channels.lock().iter().find(|c| c.id == id).cloned()
    }
}

static MAILBOX: OnceCell<Mailbox> = OnceCell::uninit();

pub fn init() {
    MAILBOX.init_once(Mailbox::new);
}

pub fn create_channel() -> ChannelId {
    MAILBOX.get().expect("Mailbox not initialized").create_channel()
}

pub fn send(channel_id: ChannelId, sender: u32, message: &[u8]) -> Result<(), &'static str> {
    let mailbox = MAILBOX.get().expect("Mailbox not initialized");
    let channel = mailbox.get_channel(channel_id).ok_or("Channel not found")?;
    channel.send(sender, message)?;
    channel.wake_one_receiver();
    Ok(())
}

pub fn send_shared_memory(
    channel_id: ChannelId,
    sender: u32,
    grant: SharedMemoryGrant,
) -> Result<(), &'static str> {
    let mailbox = MAILBOX.get().expect("Mailbox not initialized");
    let channel = mailbox.get_channel(channel_id).ok_or("Channel not found")?;
    channel.send_shared_memory(sender, grant)?;
    channel.wake_one_receiver();
    Ok(())
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

pub fn register_receiver_waiter(channel_id: ChannelId, task_id: u64) -> Result<(), &'static str> {
    let mailbox = MAILBOX.get().expect("Mailbox not initialized");
    let channel = mailbox.get_channel(channel_id).ok_or("Channel not found")?;
    channel.register_receiver_waiter(task_id);
    Ok(())
}

pub fn send_message(
    channel_id: ChannelId,
    message_ptr: *const u8,
    message_len: usize,
) -> Result<(), &'static str> {
    if !crate::caps::Capability::IpcManage.check_current() {
        return Err("Permission denied: No IpcManage capability");
    }

    if message_len > MAX_INLINE_MESSAGE_SIZE {
        return Err("Message too large");
    }

    let mut message = vec![0u8; message_len];
    copy_from_user(&mut message, message_ptr)?;

    let sender = crate::task::scheduler::get_current_task_id() as u32;
    send(channel_id, sender, &message)
}

pub fn inject_hardware_event(
    channel_id: ChannelId,
    irq: u8,
    payload: &[u8],
) -> Result<(), &'static str> {
    send(channel_id, irq as u32, payload)
}

pub fn recv_message(
    channel_id: ChannelId,
    buffer_ptr: *mut u8,
    buffer_len: usize,
    blocking: bool,
) -> Result<usize, &'static str> {
    if !crate::caps::Capability::IpcManage.check_current() {
        return Err("Permission denied: No IpcManage capability");
    }

    loop {
        let Some(message) = recv(channel_id) else {
            if !blocking {
                return Ok(0);
            }
            crate::task::block_current_on_channel(channel_id);
            continue;
        };

        match message.payload {
            MessagePayload::Inline(data) => {
                if data.len() > buffer_len {
                    return Err("Buffer too small");
                }
                copy_to_user(buffer_ptr, &data)?;
                return Ok(data.len());
            }
            MessagePayload::SharedMemory(_grant) => {
                return Err("Message is shared-memory payload; inline recv buffer is incompatible")
            }
        }
    }
}

pub fn ensure_channel(channel_id: ChannelId) -> ChannelId {
    MAILBOX
        .get()
        .expect("Mailbox not initialized")
        .create_or_get_channel(channel_id)
}
