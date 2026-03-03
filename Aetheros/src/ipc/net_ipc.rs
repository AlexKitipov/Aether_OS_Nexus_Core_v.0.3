
// src/ipc/net_ipc.rs

extern crate alloc;
use alloc::vec::Vec;

use serde::{Deserialize, Serialize};

// IPC message format for data plane operations between net-bridge and aethernet-service
#[derive(Debug, Serialize, Deserialize)]
pub enum NetPacketMsg {
    /// Sent from net-bridge to aethernet-service when a packet is received.
    /// Contains the DMA handle and the length of the received packet.
    RxPacket {
        dma_handle: u64,
        len: u64,
    },
    /// Sent from aethernet-service to net-bridge when smoltcp wants to transmit a packet.
    /// Contains the DMA handle and the length of the packet to transmit.
    TxPacket {
        dma_handle: u64,
        len: u64,
    },
    /// Acknowledgment from net-bridge after processing a TxPacket.
    TxPacketAck,
}

// IPC API for other V-Nodes (Socket API)
#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub enum NetStackRequest {
    OpenSocket(u32, u16), // type (0=TCP, 1=UDP), local_port (0 for ephemeral)
    Send(u32, Vec<u8>), // socket_handle, data
    SendTo(u32, [u8; 4], u16, Vec<u8>), // socket_handle, remote_ip, remote_port, data (new variant)
    Recv(u32), // socket_handle
    CloseSocket(u32), // socket_handle
}

#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub enum NetStackResponse {
    SocketOpened(u32), // socket_handle
    Data(Vec<u8>),
    Error(u32), // error_code
    Success,
}
