extern crate alloc;

use alloc::string::String;
use alloc::vec::Vec;
#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

/// Represents a socket file descriptor within the socket-api V-Node.
pub type SocketFd = u32;

/// Socket-related requests shared across V-Nodes.
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub enum SocketRequest {
    Socket { domain: i32, ty: i32, protocol: i32 },
    Bind { fd: SocketFd, addr: [u8; 4], port: u16 },
    Listen { fd: SocketFd, backlog: i32 },
    Accept { fd: SocketFd },
    Connect { fd: SocketFd, addr: [u8; 4], port: u16 },
    Send { fd: SocketFd, data: Vec<u8> },
    Recv { fd: SocketFd, len: u32 },
    Close { fd: SocketFd },
}

/// Socket-related responses shared across V-Nodes.
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub enum SocketResponse {
    Success(i32),
    Data(Vec<u8>),
    Error(i32, String),
    Accepted {
        new_fd: SocketFd,
        remote_addr: [u8; 4],
        remote_port: u16,
    },
}
