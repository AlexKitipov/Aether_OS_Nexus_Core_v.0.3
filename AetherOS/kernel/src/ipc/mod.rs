pub mod mailbox;

pub use aetheros_common::channel::id::ChannelId;
pub use mailbox::peek as kernel_peek;
pub use mailbox::recv as kernel_recv;
pub use mailbox::send as kernel_send;
pub use mailbox::{Message, MessagePayload, SharedMemoryGrant};

pub fn init() {
    mailbox::init();
}
