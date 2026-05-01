pub mod mailbox;

pub use aetheros_common::channel::id::ChannelId;
pub use mailbox::peek as kernel_peek;
pub use mailbox::recv as kernel_recv;
pub use mailbox::send as kernel_send;
pub use mailbox::{Message, MessagePayload, SharedMemoryGrant};

pub fn init() {
    mailbox::init();
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
