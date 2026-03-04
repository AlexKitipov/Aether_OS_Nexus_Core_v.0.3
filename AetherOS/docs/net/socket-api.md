
# Socket API (svc://socket-api)

## Overview

The `socket-api` V-Node provides a POSIX-like socket API layer for applications within AetherOS. It acts as an intermediary, translating standard socket calls into Inter-Process Communication (IPC) messages for the underlying `svc://aethernet-service` (the network stack V-Node). This design ensures strict isolation and capability-based security, as applications do not directly interact with the network stack.

## IPC Protocol

Communication with the `socket-api` V-Node occurs via IPC, using the `SocketRequest` and `SocketResponse` enums defined in `src/ipc/socket_ipc.rs`.

### SocketFd

`pub type SocketFd = u32;`

Represents a unique identifier for an open socket, analogous to a file descriptor.

### SocketRequest Enum (Client -> socket-api)

Applications send these requests to `svc://socket-api` to perform network operations.

```rust
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
```

**Parameters:**

*   `domain`: Address family (e.g., `2` for AF_INET/IPv4). Future versions may support AF_INET6 or AF_UNIX.
*   `ty`: Socket type (e.g., `1` for SOCK_STREAM/TCP, `2` for SOCK_DGRAM/UDP).
*   `protocol`: Protocol type (e.g., `0` for IP, future versions may specify specific protocols like IPPROTO_TCP/UDP).
*   `fd`: The `SocketFd` returned by a previous `Socket` request.
*   `addr`: An array of 4 bytes representing an IPv4 address (e.g., `[127, 0, 0, 1]` for localhost).
*   `port`: A 16-bit unsigned integer representing the port number.
*   `backlog`: The maximum length of the queue of pending connections.
*   `data`: A vector of bytes representing the data to send.
*   `len`: The maximum number of bytes to receive.

### SocketResponse Enum (socket-api -> Client)

`svc://socket-api` sends these responses back to the client V-Node.

```rust
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
```

**Return Values:**

*   `Success(i32)`: A successful operation. The `i32` usually contains a new `SocketFd` (for `Socket` and `Accept`), `0` for other successful operations, or the number of bytes sent/received.
*   `Data(Vec<u8>)`: The data received from a `Recv` operation.
*   `Error(i32, String)`: An error occurred. The `i32` contains an `errno`-like error code, and the `String` provides a human-readable message.
*   `Accepted { new_fd: SocketFd, remote_addr: [u8; 4], remote_port: u16 }`: Returned by `Accept` with the new client socket's file descriptor and the remote client's address and port.

## Usage Examples

### Example 1: Creating a TCP Socket

```rust
// Pseudocode for client V-Node

let mut socket_api_chan = VNodeChannel::new(4); // IPC Channel to svc://socket-api

// Request to create a TCP (SOCK_STREAM) socket
let request = SocketRequest::Socket { domain: 2, ty: 1, protocol: 0 }; // AF_INET, SOCK_STREAM
match socket_api_chan.send_and_recv::<SocketRequest, SocketResponse>(&request) {
    Ok(SocketResponse::Success(fd)) => {
        log!("TCP socket created with fd: {}", fd);
        // Now 'fd' can be used for bind, listen, connect, send, recv
    },
    Ok(SocketResponse::Error(errno, msg)) => {
        log!("Failed to create socket: {} ({})", msg, errno);
    },
    _ => log!("Unexpected response"),
}
```

### Example 2: Binding a UDP Socket and Sending Data

```rust
// Pseudocode for client V-Node

let mut socket_api_chan = VNodeChannel::new(4);

// Create UDP socket first (fd = 5, for example)
// ... (code to create UDP socket and get fd = 5)
let udp_fd: SocketFd = 5;

// Bind to a local address and port (e.g., 0.0.0.0:12345)
let bind_request = SocketRequest::Bind { fd: udp_fd, addr: [0, 0, 0, 0], port: 12345 };
match socket_api_chan.send_and_recv::<SocketRequest, SocketResponse>(&bind_request) {
    Ok(SocketResponse::Success(0)) => {
        log!("UDP socket bound to 0.0.0.0:12345");

        // Prepare data to send
        let data_to_send = b"Hello AetherOS UDP!".to_vec();

        // Send data to a remote address (e.g., 10.0.2.1:8080)
        let send_request = SocketRequest::Connect { fd: udp_fd, addr: [10, 0, 2, 1], port: 8080 }; // Connect for UDP sets default peer
        match socket_api_chan.send_and_recv::<SocketRequest, SocketResponse>(&send_request) {
            Ok(SocketResponse::Success(0)) => {
                let actual_send_request = SocketRequest::Send { fd: udp_fd, data: data_to_send };
                match socket_api_chan.send_and_recv::<SocketRequest, SocketResponse>(&actual_send_request) {
                    Ok(SocketResponse::Success(bytes_sent)) => {
                        log!("Sent {} bytes via UDP.", bytes_sent);
                    },
                    _ => log!("Failed to send UDP data."),
                }
            },
            _ => log!("Failed to set UDP remote address."),
        }
    },
    _ => log!("Failed to bind UDP socket."),
}
```

## Error Handling

Client V-Nodes should always check for `SocketResponse::Error` and handle the returned `errno` and error message appropriately. Common `errno` values might include:

*   `-1` (Generic error)
*   `11` (EWOULDBLOCK - operation would block, for non-blocking sockets)
*   `9` (EBADF - bad file descriptor)
*   `100` (Custom `socket-api` error - invalid socket type, etc.)

This API provides the necessary abstraction for applications to interact with the network, ensuring the modularity and security principles of AetherOS.
