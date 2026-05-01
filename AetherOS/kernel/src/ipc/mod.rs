pub mod mailbox;

use alloc::vec::Vec;
use conquer_once::spin::OnceCell;

pub use aetheros_common::channel::id::ChannelId;
pub use mailbox::peek as kernel_peek;
pub use mailbox::recv as kernel_recv;
pub use mailbox::send as kernel_send;
pub use mailbox::{Message, MessagePayload, SharedMemoryGrant};

static KERNEL_GATEWAY_CHANNEL: OnceCell<ChannelId> = OnceCell::uninit();

#[derive(Clone)]
pub struct KernelEvent {
    pub event_type: &'static str,
    pub payload: Vec<u8>,
}

pub fn init() {
    mailbox::init();
    let channel = mailbox::create_channel();
    KERNEL_GATEWAY_CHANNEL.init_once(|| channel);
}

pub fn gateway_channel() -> Option<ChannelId> {
    KERNEL_GATEWAY_CHANNEL.get().copied()
}

pub fn ipc_send(target_pid: u64, bytes: &[u8]) -> Result<(), &'static str> {
    if !crate::caps::Capability::IpcManage.check_current() {
        return Err("IPC capability denied");
    }

    let sender = crate::task::scheduler::get_current_task_id() as u32;
    let _ = target_pid;
    let channel = gateway_channel().ok_or("Gateway channel not initialized")?;
    mailbox::send(channel, sender, bytes)
}

pub fn ipc_receive(buffer: &mut [u8]) -> Result<usize, &'static str> {
    if !crate::caps::Capability::IpcManage.check_current() {
        return Err("IPC capability denied");
    }

    let channel = gateway_channel().ok_or("Gateway channel not initialized")?;
    let message = mailbox::recv(channel).ok_or("No message available")?;
    match message.payload {
        MessagePayload::Inline(data) => {
            if data.len() > buffer.len() {
                return Err("Receive buffer too small");
            }
            buffer[..data.len()].copy_from_slice(&data);
            Ok(data.len())
        }
        MessagePayload::SharedMemory(grant) => {
            let data = grant.as_slice();
            if data.len() > buffer.len() {
                return Err("Receive buffer too small");
            }
            buffer[..data.len()].copy_from_slice(data);
            Ok(data.len())
        }
    }
}

pub fn kernel_emit_event(event_type: &'static str, payload: &[u8]) -> Result<(), &'static str> {
    let mut bytes = Vec::with_capacity(event_type.len() + payload.len() + 1);
    bytes.extend_from_slice(event_type.as_bytes());
    bytes.push(0);
    bytes.extend_from_slice(payload);
    ipc_send(0, &bytes)
}

pub fn kernel_apply_action(payload: &[u8]) -> Result<(), &'static str> {
    if payload.is_empty() {
        return Err("Empty action payload");
    }
    Ok(())
}

pub fn send_message(channel_id: ChannelId, target_task_id: u64, payload: &[u8]) -> Result<(), &'static str> {
    if !crate::caps::Capability::IpcManage.check_current() {
        return Err("IPC capability denied");
    }

    let sender = crate::task::scheduler::get_current_task_id() as u32;
    let _ = target_task_id;
    mailbox::send(channel_id, sender, payload)
}

pub fn receive_message(channel_id: ChannelId) -> Result<Message, &'static str> {
    if !crate::caps::Capability::IpcManage.check_current() {
        return Err("IPC capability denied");
    }

    mailbox::recv(channel_id).ok_or("No message available")
}
