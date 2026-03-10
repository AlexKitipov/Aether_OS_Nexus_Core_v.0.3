pub mod mailbox;

pub use mailbox::{recv as kernel_recv, peek as kernel_peek, send as kernel_send, ChannelId, Message};

pub fn init() {
    mailbox::init();
}
