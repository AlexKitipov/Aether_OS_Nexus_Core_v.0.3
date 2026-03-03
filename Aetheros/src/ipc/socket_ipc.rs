
// src/ipc/socket_ipc.rs

#![no_std]

extern crate alloc;
use alloc::vec::Vec;
use alloc::string::String;

use serde::{Deserialize, Serialize};

/// Represents a socket file descriptor within the socket-api V-Node.
pub type SocketFd = u32;

/// Represents requests from client V-Nodes to the socket-api V-Node.
#[derive(Debug, Serialize, Deserialize)]
pub enum SocketRequest {
    /// Create a new socket.
    Socket { domain: i32, ty: i32, protocol: i32 },
    /// Bind a socket to a local address.
    Bind { fd: SocketFd, addr: [u8; 4], port: u16 },
    /// Start listening for incoming connections on a socket.
    Listen { fd: SocketFd, backlog: i32 },
    /// Accept a new connection on a listening socket.
    Accept { fd: SocketFd },
    /// Connect a socket to a remote address.
    Connect { fd: SocketFd, addr: [u8; 4], port: u16 },
    /// Send data over a socket.
    Send { fd: SocketFd, data: Vec<u8> },
    /// Receive data from a socket.
    Recv { fd: SocketFd, len: u32 },
    /// Close a socket.
    Close { fd: SocketFd },
}

/// Represents responses from the socket-api V-Node to client V-Nodes.
#[derive(Debug, Serialize, Deserialize)]
pub enum SocketResponse {
    /// Indicates success, often with a return value (e.g., new fd).
    Success(i32),
    /// Returns data received from a socket.
    Data(Vec<u8>),
    /// Indicates an error occurred.
    Error(i32, String), // errno, error_message
    /// For accept, returns the new socket fd and remote address/port.
    Accepted { new_fd: SocketFd, remote_addr: [u8; 4], remote_port: u16 },
}
