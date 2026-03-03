
// src/ipc/mail_ipc.rs

#![no_std]

extern crate alloc;
use alloc::vec::Vec;
use alloc::string::String;
use alloc::collections::BTreeMap;

use serde::{Deserialize, Serialize};

/// Represents requests from client V-Nodes to the Mail V-Node.
#[derive(Debug, Serialize, Deserialize)]
pub enum MailRequest {
    /// Send a new mail message.
    SendMail {
        recipient: String,
        subject: String,
        body: String,
    },
    /// List available mailboxes for the current user.
    ListMailboxes,
    /// Read a specific message from a given mailbox.
    ReadMessage {
        mailbox: String,
        message_id: u32,
    },
}

/// Represents responses from the Mail V-Node to client V-Nodes.
#[derive(Debug, Serialize, Deserialize)]
pub enum MailResponse {
    /// Indicates a successful operation, with an optional descriptive message.
    Success(String),
    /// Returns a list of mailbox names.
    Mailboxes(Vec<String>),
    /// Returns the content of a specific message.
    Message(String),
    /// Indicates an error occurred during the operation.
    Error(String),
}
