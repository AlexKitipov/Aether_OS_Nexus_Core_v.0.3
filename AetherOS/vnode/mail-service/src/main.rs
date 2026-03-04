// vnode/mail-service/src/main.rs

#![no_std]
#![no_main]

extern crate alloc;

use core::panic::PanicInfo;
use alloc::vec::Vec;
use alloc::collections::BTreeMap;
use alloc::format;
use alloc::string::{String, ToString};

use common::ipc::vnode::VNodeChannel;
use common::syscall::{syscall3, SYS_LOG, SUCCESS, SYS_TIME};
use common::ipc::mail_ipc::{MailRequest, MailResponse};
use common::ipc::vfs_ipc::{VfsRequest, VfsResponse, Fd, VfsMetadata};
use common::ipc::socket_ipc::{SocketRequest, SocketResponse, SocketFd};
use common::ipc::dns_ipc::{DnsRequest, DnsResponse};

// Temporary log function for V-Nodes
fn log(msg: &str) {
    unsafe {
        let res = syscall3(
            SYS_LOG,
            msg.as_ptr() as u64,
            msg.len() as u64,
            0 // arg3 is unused for SYS_LOG
        );
        if res != SUCCESS { /* Handle log error, maybe panic or fall back */ }
    }
}

// Placeholder for Mailbox and Message storage
// In a real system, messages would be stored as files in VFS
struct Mailbox {
    messages: BTreeMap<u32, String>, // message_id -> message_content
    next_message_id: u32,
}

impl Mailbox {
    fn new() -> Self {
        Self {
            messages: BTreeMap::new(),
            next_message_id: 1,
        }
    }

    fn add_message(&mut self, content: String) -> u32 {
        let id = self.next_message_id;
        self.messages.insert(id, content);
        self.next_message_id += 1;
        id
    }
}

struct MailService {
    client_chan: VNodeChannel, // Channel for AetherTerminal or other client V-Nodes
    vfs_chan: VNodeChannel, // Channel to svc://vfs for local mail storage
    socket_chan: VNodeChannel, // Channel to svc://socket-api for network mail protocols
    dns_chan: VNodeChannel, // Channel to svc://dns-resolver for mail server lookups

    // Conceptual local mail storage for the user
    // In a real system, this would be backed by VFS operations directly.
    user_mailboxes: BTreeMap<String, Mailbox>, // mailbox_name -> Mailbox
}

impl MailService {
    fn new(client_chan_id: u32, vfs_chan_id: u32, socket_chan_id: u32, dns_chan_id: u32) -> Self {
        let client_chan = VNodeChannel::new(client_chan_id);
        let vfs_chan = VNodeChannel::new(vfs_chan_id);
        let socket_chan = VNodeChannel::new(socket_chan_id);
        let dns_chan = VNodeChannel::new(dns_chan_id);

        log("Mail Service: Initializing...");

        // Conceptual: Initialize user's default mailboxes (e.g., Inbox, Sent)
        let mut user_mailboxes = BTreeMap::new();
        user_mailboxes.insert("Inbox".to_string(), Mailbox::new());
        user_mailboxes.insert("Sent".to_string(), Mailbox::new());

        Self {
            client_chan,
            vfs_chan,
            socket_chan,
            dns_chan,
            user_mailboxes,
        }
    }

    fn handle_request(&mut self, request: MailRequest) -> MailResponse {
        match request {
            MailRequest::SendMail { recipient, subject, body } => {
                log(&alloc::format!("Mail: Sending mail to {}: Subject: {}.", recipient, subject));
                
                // Conceptual: Resolve recipient's mail server via DNS
                // let mail_server_hostname = "smtp.example.com"; // Derived from recipient
                // match self.dns_chan.send_and_recv::<DnsRequest, DnsResponse>(&DnsRequest::ResolveHostname { hostname: mail_server_hostname.to_string() }) {
                //     Ok(DnsResponse::ResolvedHostname { ip_address, .. }) => {
                //         log!("Resolved mail server to: {:?}", ip_address);
                //         // Conceptual: Open socket connection and send mail via SMTP commands
                //         // For now, just simulate success.
                //         MailResponse::Success(alloc::format!("Mail to {} sent successfully (conceptual).", recipient))
                //     },
                //     _ => MailResponse::Error(alloc::format!("Failed to resolve mail server for {}.", recipient)),
                // }

                // Simulate storing a copy in 'Sent' mailbox
                let full_message = alloc::format!("To: {}\nSubject: {}\n\n{}", recipient, subject, body);
                if let Some(mailbox) = self.user_mailboxes.get_mut("Sent") {
                    mailbox.add_message(full_message);
                    log("Mail: Stored copy in 'Sent' mailbox.");
                }

                MailResponse::Success(alloc::format!("Mail to {} sent successfully (conceptual).", recipient))
            },
            MailRequest::ListMailboxes => {
                log("Mail: Listing mailboxes.");
                // Conceptual: Interact with VFS to list directories under /home/<AID>/mail/
                let mailboxes: Vec<String> = self.user_mailboxes.keys().cloned().collect();
                MailResponse::Mailboxes(mailboxes)
            },
            MailRequest::ReadMessage { mailbox, message_id } => {
                log(&alloc::format!("Mail: Reading message {} from mailbox {}.", message_id, mailbox));
                // Conceptual: Interact with VFS to read file content from /home/<AID>/mail/<mailbox>/<message_id>.msg
                if let Some(mb) = self.user_mailboxes.get(&mailbox) {
                    if let Some(message) = mb.messages.get(&message_id) {
                        MailResponse::Message(message.clone())
                    } else {
                        MailResponse::Error(alloc::format!("Message {} not found in mailbox {}.", message_id, mailbox))
                    }
                } else {
                    MailResponse::Error(alloc::format!("Mailbox {} not found.", mailbox))
                }
            },
        }
    }

    fn run_loop(&mut self) -> ! {
        log("Mail Service: Entering main event loop.");
        loop {
            // Process incoming requests from client V-Nodes
            if let Ok(Some(req_data)) = self.client_chan.recv_non_blocking() {
                if let Ok(request) = postcard::from_bytes::<MailRequest>(&req_data) {
                    log(&alloc::format!("Mail Service: Received MailRequest: {:?}.", request));
                    let response = self.handle_request(request);
                    self.client_chan.send(&response).unwrap_or_else(|_| log("Mail Service: Failed to send response to client."));
                } else {
                    log("Mail Service: Failed to deserialize MailRequest.");
                }
            }

            // Conceptual: Periodically check for new incoming mail (via socket-api, DNS)
            // This would involve polling a mail server (e.g., POP3, IMAP).

            // Yield to other V-Nodes to prevent busy-waiting
            unsafe { syscall3(SYS_TIME, 0, 0, 0); } // This will cause a context switch
        }
    }
}

#[no_mangle]
pub extern "C" fn _start() -> ! {
    // Assuming channel IDs:
    // 10 for Mail Service client requests
    // 7 for VFS Service
    // 4 for Socket API Service
    // 5 for DNS Resolver Service
    let mut mail_service = MailService::new(10, 7, 4, 5);
    mail_service.run_loop();
}

#[panic_handler]
pub extern "C" fn panic(info: &PanicInfo) -> ! {
    log(&alloc::format!("Mail V-Node panicked! Info: {:?}.", info));
    // In a production system, this might trigger a system-wide error handler or reboot.
    // For now, it enters an infinite loop to prevent further execution.
    loop {}
}
