// vnode/mail-service/src/main.rs

#![no_std]
#![no_main]

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::format;
use alloc::string::{String, ToString};
use alloc::vec::Vec;
use core::panic::PanicInfo;
use linked_list_allocator::LockedHeap;

use common::ipc::dns_ipc::{DnsRequest, DnsResponse};
use common::ipc::mail_ipc::{MailRequest, MailResponse};
use common::ipc::socket_ipc::{SocketRequest, SocketResponse};
use common::ipc::vfs_ipc::{VfsRequest, VfsResponse};
use common::ipc::vnode::VNodeChannel;
use common::syscall::{syscall3, SUCCESS, SYS_LOG, SYS_TIME};

const VNODE_HEAP_SIZE: usize = 64 * 1024;
static mut VNODE_HEAP: [u8; VNODE_HEAP_SIZE] = [0; VNODE_HEAP_SIZE];

#[global_allocator]
static GLOBAL_ALLOCATOR: LockedHeap = LockedHeap::empty();

fn init_allocator() {
    unsafe {
        GLOBAL_ALLOCATOR
            .lock()
            .init(VNODE_HEAP.as_mut_ptr(), VNODE_HEAP_SIZE);
    }
}

fn log(msg: &str) {
    unsafe {
        let res = syscall3(SYS_LOG, msg.as_ptr() as u64, msg.len() as u64, 0);
        if res != SUCCESS {
            // Ignore transient logging failures inside service loop.
        }
    }
}

struct Mailbox {
    messages: BTreeMap<u32, String>,
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
    client_chan: VNodeChannel,
    vfs_chan: VNodeChannel,
    socket_chan: VNodeChannel,
    dns_chan: VNodeChannel,
    user_mailboxes: BTreeMap<String, Mailbox>,
}

impl MailService {
    fn new(client_chan_id: u32, vfs_chan_id: u32, socket_chan_id: u32, dns_chan_id: u32) -> Self {
        let client_chan = VNodeChannel::new(client_chan_id);
        let vfs_chan = VNodeChannel::new(vfs_chan_id);
        let socket_chan = VNodeChannel::new(socket_chan_id);
        let dns_chan = VNodeChannel::new(dns_chan_id);

        log("Mail Service: Initializing...");

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

    fn derive_mail_host(recipient: &str) -> String {
        recipient
            .split('@')
            .nth(1)
            .map(|domain| format!("mx.{}", domain))
            .unwrap_or_else(|| String::from("mx.local"))
    }

    fn resolve_mail_host(&mut self, recipient: &str) -> Option<[u8; 4]> {
        let hostname = Self::derive_mail_host(recipient);
        let req = DnsRequest::ResolveHostname {
            hostname: hostname.clone(),
        };
        match self.dns_chan.send_and_recv::<DnsRequest, DnsResponse>(&req) {
            Ok(DnsResponse::ResolvedHostname { ip_address, .. }) => Some(ip_address),
            Ok(DnsResponse::NotFound { .. }) => {
                log("Mail: DNS host not found.");
                None
            }
            Ok(DnsResponse::Error { message }) => {
                log(&format!("Mail: DNS error: {}", message));
                None
            }
            Err(_) => {
                log(&format!("Mail: DNS lookup failed for {}", hostname));
                None
            }
        }
    }

    fn relay_via_socket(&mut self, ip: [u8; 4], wire_message: &[u8]) {
        let socket_fd = match self
            .socket_chan
            .send_and_recv::<SocketRequest, SocketResponse>(&SocketRequest::Socket {
                domain: 2,
                ty: 1,
                protocol: 0,
            }) {
            Ok(SocketResponse::Success(fd)) if fd >= 0 => fd as u32,
            _ => {
                log("Mail: socket open failed; continuing in local-only mode.");
                return;
            }
        };

        let connect = SocketRequest::Connect {
            fd: socket_fd,
            addr: ip,
            port: 2525,
        };
        let _ = self
            .socket_chan
            .send_and_recv::<SocketRequest, SocketResponse>(&connect);

        let send_req = SocketRequest::Send {
            fd: socket_fd,
            data: wire_message.to_vec(),
        };
        let _ = self
            .socket_chan
            .send_and_recv::<SocketRequest, SocketResponse>(&send_req);

        let _ = self
            .socket_chan
            .send_and_recv::<SocketRequest, SocketResponse>(&SocketRequest::Close { fd: socket_fd });
    }

    fn persist_sent_to_vfs(&mut self, message_id: u32, wire_message: &[u8]) {
        let path = format!("/home/user/mail/Sent/{}.msg", message_id);
        let fd = match self
            .vfs_chan
            .send_and_recv::<VfsRequest, VfsResponse>(&VfsRequest::Open {
                path,
                flags: 0x041, // O_WRONLY|O_CREAT
            }) {
            Ok(VfsResponse::Success(raw_fd)) if raw_fd >= 0 => raw_fd as u32,
            _ => {
                log("Mail: VFS open failed; retained in-memory only.");
                return;
            }
        };

        let _ = self
            .vfs_chan
            .send_and_recv::<VfsRequest, VfsResponse>(&VfsRequest::Write {
                fd,
                data: wire_message.to_vec(),
                offset: 0,
            });
        let _ = self
            .vfs_chan
            .send_and_recv::<VfsRequest, VfsResponse>(&VfsRequest::Close { fd });
    }

    fn handle_request(&mut self, request: MailRequest) -> MailResponse {
        match request {
            MailRequest::SendMail {
                recipient,
                subject,
                body,
            } => {
                log(&format!("Mail: Sending mail to {}", recipient));

                let wire_message = format!("To: {}\nSubject: {}\n\n{}", recipient, subject, body);
                let mut sent_id = 0;
                if let Some(mailbox) = self.user_mailboxes.get_mut("Sent") {
                    sent_id = mailbox.add_message(wire_message.clone());
                }

                if let Some(ip) = self.resolve_mail_host(&recipient) {
                    self.relay_via_socket(ip, wire_message.as_bytes());
                }
                if sent_id != 0 {
                    self.persist_sent_to_vfs(sent_id, wire_message.as_bytes());
                }

                MailResponse::Success(format!(
                    "Mail to {} queued and persisted as message #{}.",
                    recipient, sent_id
                ))
            }
            MailRequest::ListMailboxes => {
                let mailboxes: Vec<String> = self.user_mailboxes.keys().cloned().collect();
                MailResponse::Mailboxes(mailboxes)
            }
            MailRequest::ReadMessage {
                mailbox,
                message_id,
            } => {
                if let Some(mb) = self.user_mailboxes.get(&mailbox) {
                    if let Some(message) = mb.messages.get(&message_id) {
                        MailResponse::Message(message.clone())
                    } else {
                        MailResponse::Error(format!(
                            "Message {} not found in mailbox {}.",
                            message_id, mailbox
                        ))
                    }
                } else {
                    MailResponse::Error(format!("Mailbox {} not found.", mailbox))
                }
            }
        }
    }

    fn run_loop(&mut self) -> ! {
        log("Mail Service: Entering main event loop.");
        loop {
            if let Ok(Some(req_data)) = self.client_chan.recv_non_blocking() {
                if let Ok(request) = postcard::from_bytes::<MailRequest>(&req_data) {
                    let response = self.handle_request(request);
                    self.client_chan
                        .send(&response)
                        .unwrap_or_else(|_| log("Mail Service: Failed to send response to client."));
                } else {
                    log("Mail Service: Failed to deserialize MailRequest.");
                }
            }

            unsafe {
                syscall3(SYS_TIME, 0, 0, 0);
            }
        }
    }
}

#[no_mangle]
pub extern "C" fn _start() -> ! {
    init_allocator();
    let mut mail_service = MailService::new(10, 7, 4, 5);
    mail_service.run_loop();
}

#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    log(&format!("Mail V-Node panicked! Info: {:?}.", info));
    loop {}
}
