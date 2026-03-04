
# Mail/Messaging V-Node (svc://mail-service)

## Overview

The `mail-service` V-Node provides a centralized messaging and mail management service for AetherOS users and applications. It acts as an intermediary for sending and receiving electronic mail, managing user mailboxes, and interacting with network mail protocols (like SMTP, POP3, IMAP) through the `svc://socket-api` V-Node. This V-Node leverages the `svc://vfs` for persistent storage of mailboxes and messages, ensuring data integrity and user control.

## IPC Protocol

Communication with the `mail-service` V-Node occurs via IPC, using the `MailRequest` and `MailResponse` enums defined in `src/ipc/mail_ipc.rs`.

### MailRequest Enum (Client -> mail-service)

Client V-Nodes (e.g., a mail client application) send these requests to `svc://mail-service` to perform mail operations.

```rust
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
```

**Parameters:**

*   `recipient`: A `String` representing the email address of the recipient.
*   `subject`: A `String` representing the subject line of the email.
*   `body`: A `String` containing the main content of the email.
*   `mailbox`: A `String` representing the name of the mailbox (e.g., "Inbox", "Sent").
*   `message_id`: A `u32` representing the unique identifier of a message within a mailbox.

### MailResponse Enum (mail-service -> Client)

`svc://mail-service` sends these responses back to the client V-Node after processing a `MailRequest`.

```rust
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
```

**Return Values:**

*   `Success(String)`: A successful operation, with an optional descriptive message.
*   `Mailboxes(Vec<String>)`: A vector of strings, each representing a mailbox name.
*   `Message(String)`: The full content of a requested message.
*   `Error(String)`: An error occurred during the operation, with a descriptive message.

## Functionality

The `mail-service` V-Node performs the following key functions:

1.  **IPC Interface**: Exposes a well-defined IPC interface for client applications to request mail management actions.
2.  **Mailbox Management**: Manages user mailboxes (e.g., Inbox, Sent, Drafts), conceptually backed by the VFS at `/home/<AID>/mail/`.
3.  **Message Storage**: Stores and retrieves mail messages, typically as files within specific mailbox directories in the VFS.
4.  **Network Integration**: Interacts with `svc://socket-api` to send outgoing mail via SMTP and receive incoming mail via protocols like POP3 or IMAP. It also uses `svc://dns-resolver` to look up mail server hostnames.
5.  **Error Handling**: Translates errors from underlying VFS or network operations into standardized `MailResponse::Error` messages.
6.  **User Context**: (Conceptual) Integrates with the user's Aether Identity (AID) for personalized mail storage and authentication with mail servers.

## Usage Examples

### Example 1: Sending a Mail Message

```rust
// Pseudocode for client V-Node (e.g., a mail client GUI) wanting to send an email

let mut mail_chan = VNodeChannel::new(10); // IPC Channel to svc://mail-service

let request = MailRequest::SendMail {
    recipient: String::from("user@example.com"),
    subject: String::from("Hello AetherOS!"),
    body: String::from("This is a test message from AetherOS."),
};
match mail_chan.send_and_recv::<MailRequest, MailResponse>(&request) {
    Ok(MailResponse::Success(msg)) => {
        log!("Mail sent successfully: {}", msg);
    },
    Ok(MailResponse::Error(msg)) => {
        log!("Failed to send mail: {}", msg);
    },
    _ => log!("Unexpected response from Mail Service"),
}
```

### Example 2: Listing Mailboxes

```rust
// Pseudocode for client V-Node wanting to list available mailboxes

let mut mail_chan = VNodeChannel::new(10);

let request = MailRequest::ListMailboxes;
match mail_chan.send_and_recv::<MailRequest, MailResponse>(&request) {
    Ok(MailResponse::Mailboxes(mailboxes)) => {
        log!("Available mailboxes: {:?}", mailboxes);
    },
    Ok(MailResponse::Error(msg)) => {
        log!("Failed to list mailboxes: {}", msg);
    },
    _ => log!("Unexpected response from Mail Service"),
}
```

### Example 3: Reading a Message

```rust
// Pseudocode for client V-Node wanting to read a specific message

let mut mail_chan = VNodeChannel::new(10);

let request = MailRequest::ReadMessage {
    mailbox: String::from("Inbox"),
    message_id: 1,
};
match mail_chan.send_and_recv::<MailRequest, MailResponse>(&request) {
    Ok(MailResponse::Message(content)) => {
        log!("Message content:\n{}", content);
    },
    Ok(MailResponse::Error(msg)) => {
        log!("Failed to read message: {}", msg);
    },
    _ => log!("Unexpected response from Mail Service"),
}
```

This documentation outlines the `mail-service` V-Node's role as a vital communication hub, demonstrating its modularity and reliance on IPC for interacting with other core AetherOS services to deliver a secure and efficient messaging experience.
