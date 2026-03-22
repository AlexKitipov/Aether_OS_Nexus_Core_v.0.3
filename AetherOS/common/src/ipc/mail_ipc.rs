#![no_std]

extern crate alloc;

use alloc::string::String;
use alloc::vec::Vec;
use serde::{Deserialize, Serialize};

/// Requests sent to the mail-service V-Node.
#[derive(Debug, Serialize, Deserialize)]
pub enum MailRequest {
    SendMail {
        recipient: String,
        subject: String,
        body: String,
    },
    ListMailboxes,
    ReadMessage {
        mailbox: String,
        message_id: u32,
    },
}

/// Responses returned by the mail-service V-Node.
#[derive(Debug, Serialize, Deserialize)]
pub enum MailResponse {
    Success(String),
    Mailboxes(Vec<String>),
    Message(String),
    Error(String),
}
