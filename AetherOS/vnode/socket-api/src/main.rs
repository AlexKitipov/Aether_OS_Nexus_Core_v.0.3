#![no_std]
#![no_main]

extern crate alloc;

use core::panic::PanicInfo;
use alloc::vec::Vec;
use alloc::collections::BTreeMap;
use alloc::format;
use alloc::string::{String, ToString};

use crate::ipc::vnode::VNodeChannel;
use crate::syscall::{syscall3, SYS_LOG, SUCCESS, SYS_TIME};
use crate::ipc::net_ipc::{NetStackRequest, NetStackResponse};
use crate::ipc::socket_ipc::{SocketRequest, SocketResponse, SocketFd};

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

// Placeholder for socket state (simulated file descriptor management)
#[derive(Debug, Clone)]
struct SocketInfo {
    net_socket_handle: u32, // The handle given by svc://aethernet
    socket_type: i32, // SOCK_STREAM or SOCK_DGRAM (as per SocketRequest `ty`)
    is_listening: bool,
    // Add more state as needed, e.g., remote address for connected sockets
}

#[no_mangle]
pub extern "C" fn _start() -> ! {
    // Channel for requests from client V-Nodes to this socket-api V-Node
    let mut client_chan = VNodeChannel::new(4); // Assuming channel ID 4 for socket-api

    // Channel to communicate with svc://aethernet-service
    let mut net_chan = VNodeChannel::new(3); // Assuming channel ID 3 for aethernet-service

    log("Socket API V-Node starting up...");

    let mut next_fd: SocketFd = 1;
    let mut sockets: BTreeMap<SocketFd, SocketInfo> = BTreeMap::new();
    // `pending_accept_fci` is not strictly needed if aethernet-service directly sends new connection info.
    // For now, keep it simple by returning EWOULDBLOCK for accept.

    loop {
        // 1. Process incoming requests from client V-Nodes
        if let Ok(Some(req_data)) = client_chan.recv_non_blocking() {
            if let Ok(request) = postcard::from_bytes::<SocketRequest>(&req_data) {
                log(&alloc::format!("SocketAPI: Received request from client: {:?}", request));

                let response = match request {
                    SocketRequest::Socket { domain, ty, protocol } => {
                        // For now, only AF_INET (domain 2), SOCK_STREAM (type 1), SOCK_DGRAM (type 2) are conceptual
                        // Map our type to aethernet-service's type (0=TCP, 1=UDP)
                        let net_sock_type = match ty {
                            1 => 0, // SOCK_STREAM -> TCP
                            2 => 1, // SOCK_DGRAM -> UDP
                            _ => {
                                log(&alloc::format!("SocketAPI: Unsupported socket type: {}", ty));
                                return SocketResponse::Error(100, "Unsupported socket type".to_string());
                            }
                        };

                        match net_chan.send_and_recv::<NetStackRequest, NetStackResponse>(&NetStackRequest::OpenSocket(net_sock_type, 0)) {
                            Ok(NetStackResponse::SocketOpened(net_handle)) => {
                                let fd = next_fd;
                                next_fd += 1;
                                sockets.insert(fd, SocketInfo { net_socket_handle: net_handle, socket_type: ty, is_listening: false });
                                log(&alloc::format!("SocketAPI: Opened new socket with fd: {}, net_handle: {}", fd, net_handle));
                                SocketResponse::Success(fd as i32)
                            },
                            Ok(NetStackResponse::Error(code)) => {
                                log(&alloc::format!("SocketAPI: Failed to open socket in AetherNet. Error code: {}", code));
                                SocketResponse::Error(code as i32, "Failed to open socket in AetherNet".to_string())
                            },
                            _ => {
                                log("SocketAPI: Unexpected response from AetherNet during Socket open.");
                                SocketResponse::Error(-1, "Unexpected response from AetherNet during Socket open".to_string())
                            },
                        }
                    },
                    SocketRequest::Bind { fd, addr, port } => {
                        if let Some(socket_info) = sockets.get_mut(&fd) {
                            // `aethernet-service`'s `OpenSocket` is used for both creation and binding to a local port.
                            // So, we re-call `OpenSocket` with the existing socket_type and the new local_port.
                            // This might create a new smoltcp socket and return a new handle, or reconfigure an existing one.
                            let net_sock_type = match socket_info.socket_type {
                                1 => 0, // SOCK_STREAM -> TCP
                                2 => 1, // SOCK_DGRAM -> UDP
                                _ => {
                                    log(&alloc::format!("SocketAPI: Cannot bind unsupported socket type: {}", socket_info.socket_type));
                                    return SocketResponse::Error(100, "Unsupported socket type for bind".to_string());
                                }
                            };
                            match net_chan.send_and_recv::<NetStackRequest, NetStackResponse>(&NetStackRequest::OpenSocket(net_sock_type, port)) {
                                Ok(NetStackResponse::SocketOpened(new_net_handle)) => {
                                    // Update the net_socket_handle if aethernet-service returned a new one after binding
                                    socket_info.net_socket_handle = new_net_handle;
                                    log(&alloc::format!("SocketAPI: Socket fd {} bound to {}:{}, new net_handle: {}", fd, addr[0], port, new_net_handle));
                                    SocketResponse::Success(0)
                                },
                                Ok(NetStackResponse::Error(code)) => {
                                    log(&alloc::format!("SocketAPI: Failed to bind socket fd {} in AetherNet. Error: {}", fd, code));
                                    SocketResponse::Error(code as i32, "Failed to bind socket in AetherNet".to_string())
                                },
                                _ => {
                                    log(&alloc::format!("SocketAPI: Unexpected response from AetherNet during Bind for fd {}.
", fd));
                                    SocketResponse::Error(-1, "Unexpected response from AetherNet during Bind".to_string())
                                },
                            }
                        } else {
                            log(&alloc::format!("SocketAPI: Bind failed, bad file descriptor: {}", fd));
                            SocketResponse::Error(9, "Bad file descriptor".to_string()) // EBADF
                        }
                    },
                    SocketRequest::Listen { fd, backlog: _ } => { // backlog is conceptual for smoltcp
                        if let Some(socket_info) = sockets.get_mut(&fd) {
                            // In smoltcp, `listen` is part of TcpSocket creation/configuration if a port is given.
                            // Here, we just mark our internal state as listening.
                            if socket_info.socket_type == 1 { // Only TCP sockets can listen
                                socket_info.is_listening = true;
                                log(&alloc::format!("SocketAPI: Socket fd {} marked as listening.", fd));
                                SocketResponse::Success(0)
                            } else {
                                log(&alloc::format!("SocketAPI: Socket fd {} cannot listen, not a TCP socket.", fd));
                                SocketResponse::Error(105, "Only TCP sockets can listen".to_string())
                            }
                        } else {
                            log(&alloc::format!("SocketAPI: Listen failed, bad file descriptor: {}", fd));
                            SocketResponse::Error(9, "Bad file descriptor".to_string()) // EBADF
                        }
                    },
                    SocketRequest::Accept { fd } => {
                        // This would typically involve blocking and waiting for a connection.
                        // In a non-blocking loop, aethernet-service would send an IPC message
                        // to socket-api when a connection is accepted, which socket-api would then relay.
                        // For now, it's conceptual and returns EWOULDBLOCK.
                        log(&alloc::format!("SocketAPI: Accept on fd {} is conceptual; requires AetherNet callback.", fd));
                        SocketResponse::Error(11, "Operation would block (EWOULDBLOCK)".to_string()) // EWOULDBLOCK
                    },
                    SocketRequest::Connect { fd, addr, port } => {
                        if let Some(socket_info) = sockets.get_mut(&fd) {
                            if socket_info.socket_type == 2 { // UDP
                                // For UDP, 'connect' sets the default remote peer for future `send` calls.
                                // We use `NetStackRequest::SendTo` with empty data to conceptually set the peer.
                                match net_chan.send_and_recv::<NetStackRequest, NetStackResponse>(&NetStackRequest::SendTo(socket_info.net_socket_handle, addr, port, Vec::new())) {
                                    Ok(NetStackResponse::Success) => {
                                        log(&alloc::format!("SocketAPI: UDP socket fd {} connected to {}:{}", fd, addr[0], port));
                                        SocketResponse::Success(0)
                                    },
                                    Ok(NetStackResponse::Error(code)) => {
                                        log(&alloc::format!("SocketAPI: Failed to connect UDP socket fd {} via AetherNet. Error: {}", fd, code));
                                        SocketResponse::Error(code as i32, "Failed to connect UDP socket via AetherNet".to_string())
                                    },
                                    _ => {
                                        log(&alloc::format!("SocketAPI: Unexpected response from AetherNet during UDP Connect for fd {}.", fd));
                                        SocketResponse::Error(-1, "Unexpected response from AetherNet during UDP Connect".to_string())
                                    },
                                }
                            } else if socket_info.socket_type == 1 { // TCP
                                // For TCP, this should trigger a connection handshake in AetherNet.
                                // NetStackRequest currently lacks a specific 'Connect' variant for TCP with remote_ip/port.
                                // This would require extending NetStackRequest.
                                log(&alloc::format!("SocketAPI: TCP Connect on fd {} to {}:{} is conceptual and requires NetStackRequest extension.", fd, addr[0], port));
                                SocketResponse::Error(106, "TCP Connect not fully implemented yet".to_string())
                            } else {
                                log(&alloc::format!("SocketAPI: Unsupported socket type {} for connect on fd {}.
", socket_info.socket_type, fd));
                                SocketResponse::Error(100, "Unsupported socket type for connect".to_string())
                            }
                        } else {
                            log(&alloc::format!("SocketAPI: Connect failed, bad file descriptor: {}", fd));
                            SocketResponse::Error(9, "Bad file descriptor".to_string()) // EBADF
                        }
                    },
                    SocketRequest::Send { fd, data } => {
                        if let Some(socket_info) = sockets.get(&fd) {
                            let net_req = if socket_info.socket_type == 1 { // TCP
                                NetStackRequest::Send(socket_info.net_socket_handle, data)
                            } else if socket_info.socket_type == 2 { // UDP (assuming connect has set a default peer)
                                // AetherNet's `Send` is generic enough to handle UDP send to default peer
                                NetStackRequest::Send(socket_info.net_socket_handle, data)
                            } else {
                                log(&alloc::format!("SocketAPI: Unsupported socket type {} for send on fd {}.
", socket_info.socket_type, fd));
                                return SocketResponse::Error(100, "Unsupported socket type for send".to_string());
                            };

                            match net_chan.send_and_recv::<NetStackRequest, NetStackResponse>(&net_req) {
                                Ok(NetStackResponse::Success) => {
                                    log(&alloc::format!("SocketAPI: Sent {} bytes on fd {}", data.len(), fd));
                                    SocketResponse::Success(data.len() as i32)
                                },
                                Ok(NetStackResponse::Error(code)) => {
                                    log(&alloc::format!("SocketAPI: Failed to send on fd {} via AetherNet. Error: {}", fd, code));
                                    SocketResponse::Error(code as i32, "Failed to send via AetherNet".to_string())
                                },
                                _ => {
                                    log(&alloc::format!("SocketAPI: Unexpected response from AetherNet during Send for fd {}.
", fd));
                                    SocketResponse::Error(-1, "Unexpected response from AetherNet during Send".to_string())
                                },
                            }
                        } else {
                            log(&alloc::format!("SocketAPI: Send failed, bad file descriptor: {}", fd));
                            SocketResponse::Error(9, "Bad file descriptor".to_string()) // EBADF
                        }
                    },
                    SocketRequest::Recv { fd, len: _ } => { // len is a hint, actual data len from NetStack
                        if let Some(socket_info) = sockets.get(&fd) {
                            match net_chan.send_and_recv::<NetStackRequest, NetStackResponse>(&NetStackRequest::Recv(socket_info.net_socket_handle)) {
                                Ok(NetStackResponse::Data(data)) => {
                                    log(&alloc::format!("SocketAPI: Received {} bytes on fd {}", data.len(), fd));
                                    SocketResponse::Data(data)
                                },
                                Ok(NetStackResponse::Error(code)) => {
                                    log(&alloc::format!("SocketAPI: Failed to receive on fd {} via AetherNet. Error: {}", fd, code));
                                    SocketResponse::Error(code as i32, "Failed to receive via AetherNet".to_string())
                                },
                                _ => {
                                    log(&alloc::format!("SocketAPI: Unexpected response from AetherNet during Recv for fd {}.
", fd));
                                    SocketResponse::Error(-1, "Unexpected response from AetherNet during Recv".to_string())
                                },
                            }
                        } else {
                            log(&alloc::format!("SocketAPI: Recv failed, bad file descriptor: {}", fd));
                            SocketResponse::Error(9, "Bad file descriptor".to_string()) // EBADF
                        }
                    },
                    SocketRequest::Close { fd } => {
                        if let Some(socket_info) = sockets.remove(&fd) {
                            match net_chan.send_and_recv::<NetStackRequest, NetStackResponse>(&NetStackRequest::CloseSocket(socket_info.net_socket_handle)) {
                                Ok(NetStackResponse::Success) => {
                                    log(&alloc::format!("SocketAPI: Closed socket fd {}", fd));
                                    SocketResponse::Success(0)
                                },
                                Ok(NetStackResponse::Error(code)) => {
                                    log(&alloc::format!("SocketAPI: Failed to close socket fd {} in AetherNet. Error: {}", fd, code));
                                    SocketResponse::Error(code as i32, "Failed to close socket in AetherNet".to_string())
                                },
                                _ => {
                                    log(&alloc::format!("SocketAPI: Unexpected response from AetherNet during Close for fd {}.
", fd));
                                    SocketResponse::Error(-1, "Unexpected response from AetherNet during Close".to_string())
                                },
                            }
                        } else {
                            log(&alloc::format!("SocketAPI: Close failed, bad file descriptor: {}", fd));
                            SocketResponse::Error(9, "Bad file descriptor".to_string()) // EBADF
                        }
                    },
                };
                client_chan.send(&response).unwrap_or_else(|_| log("SocketAPI: Failed to send response to client."));
            } else {
                log("SocketAPI: Failed to deserialize SocketRequest.");
            }
        }
        
        // TODO: In a more complete implementation, this V-Node would also need to monitor
        // the 'net_chan' for incoming unsolicited messages from aethernet-service (e.g.,
        // for accepted connections, or asynchronous incoming data for non-blocking sockets).

        unsafe { syscall3(SYS_TIME, 0, 0, 0); } // Yield to other V-Nodes
    }
}

#[panic_handler]
pub extern "C" fn panic(info: &PanicInfo) -> ! {
    log(&alloc::format!("Socket API V-Node panicked! Info: {:?}", info));
    loop {}
}
