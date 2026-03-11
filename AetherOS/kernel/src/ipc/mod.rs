pub mod mailbox;

pub use aetheros_common::channel::id::ChannelId;
pub use mailbox::{recv as kernel_recv, peek as kernel_peek, send as kernel_send, Message};

pub fn init() {
    mailbox::init();
}
