#![no_std]

extern crate alloc;

use alloc::vec::Vec;
use alloc::collections::VecDeque; // Added for VecDeque
use smoltcp::phy::{Device, RxToken, TxToken, Checksum, DeviceCapabilities};
use smoltcp::time::Instant;
use smoltcp::wire::{EthernetAddress, HardwareAddress};

use crate::ipc::vnode::VNodeChannel;
use crate::syscall::{syscall3, SYS_LOG, SUCCESS, E_ERROR, SYS_NET_ALLOC_BUF, SYS_NET_FREE_BUF, SYS_GET_DMA_BUF_PTR, syscall2, E_ACC_DENIED, SYS_SET_DMA_BUF_LEN};
use crate::ipc::net_ipc::NetPacketMsg;

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

// Temporary syscall for allocating a DMA buffer
pub fn net_alloc_buf(size: usize) -> Result<u64, u64> {
    unsafe {
        let handle = syscall2(SYS_NET_ALLOC_BUF, size as u64, 0);
        if handle == E_ERROR || handle == E_ACC_DENIED { Err(handle) } else { Ok(handle) }
    }
}

// Temporary syscall for freeing a DMA buffer
pub fn net_free_buf(handle: u64) -> Result<(), u64> {
    unsafe {
        let res = syscall2(SYS_NET_FREE_BUF, handle, 0);
        if res != SUCCESS { Err(res) } else { Ok(()) }
    }
}

// Temporary syscall for transmitting a network packet (used by PacketTxToken consume)
// This is publicly exposed for other internal modules if needed, but the IPC is primary.
pub fn net_tx(_iface_id: u64, _buf_handle: u64, _len: u64) -> Result<(), u64> {
    // This function is here to satisfy the instructions, but its direct use
    // for TX is largely replaced by IPC with net-bridge.
    // In a pure IPC model, this would not be called directly for TX.
    Ok( crate::syscall::SUCCESS ) // Placeholder, actual TX is done via IPC to net-bridge
}

// Temporary syscall to get pointer to DMA buffer
pub fn get_dma_buffer_ptr(handle: u64) -> Result<*mut u8, u64> {
    unsafe {
        let ptr = syscall2(SYS_GET_DMA_BUF_PTR, handle, 0);
        if ptr == E_ERROR || ptr == E_ACC_DENIED { Err(ptr) } else { Ok(ptr as *mut u8) }
    }
}

// Temporary syscall to set DMA buffer length
pub fn set_dma_buffer_len(handle: u64, len: usize) -> Result<(), u64> {
    unsafe {
        let res = syscall2(SYS_SET_DMA_BUF_LEN, handle, len as u64);
        if res != SUCCESS { Err(res) } else { Ok(()) }
    }
}

/// Represents a single received packet buffer for smoltcp.
pub struct PacketRxToken<'a> {
    buffer: &'a mut [u8],
    dma_handle: u64,
}

impl<'a> RxToken for PacketRxToken<'a> {
    fn consume<R, F>(self, _timestamp: Instant, f: F) -> R
    where
        F: FnOnce(&mut [u8]) -> R,
    {
        // The smoltcp stack consumes the packet data
        let result = f(self.buffer);
        // After consumption, free the DMA buffer
        if let Err(e) = net_free_buf(self.dma_handle) {
            log(&alloc::format!("AetherNetDevice: Failed to free RX DMA buffer: {}", e));
        }
        result
    }
}

/// Represents a single transmitted packet buffer for smoltcp.
pub struct PacketTxToken<'a> {
    buffer: &'a mut [u8],
    dma_handle: u64,
    len: usize,
    iface_id: u64,
    net_bridge_chan_id: u32, // Channel ID to net-bridge V-Node
}

impl<'a> TxToken for PacketTxToken<'a> {
    fn consume<R, F>(mut self, _timestamp: Instant, f: F) -> R
    where
        F: FnOnce(&mut [u8]) -> R,
    {
        let result = f(self.buffer);
        // After smoltcp fills the buffer, send it to net-bridge for transmission
        let mut net_bridge_chan = VNodeChannel::new(self.net_bridge_chan_id);
        let msg = NetPacketMsg::TxPacket { dma_handle: self.dma_handle, len: self.len as u64 };
        net_bridge_chan.send(&msg).unwrap_or_else(|_| log(&alloc::format!("AetherNetDevice: Failed to send TxPacket to net-bridge for handle: {}", self.dma_handle)));

        // The net-bridge V-Node will free the DMA buffer after transmission (no direct net_tx call here)
        result
    }
}

/// AetherNetDevice implements smoltcp::phy::Device for communication with net-bridge V-Node.
pub struct AetherNetDevice {
    iface_id: u64, // Interface ID, typically 0 for the first NIC
    net_bridge_chan_id: u32, // Channel ID to net-bridge V-Node for TxPacket and RxPacket
    rx_packet_queue: VecDeque<(u64, u64)>, // Queue of (dma_handle, len) for received packets
}

impl AetherNetDevice {
    pub fn new(iface_id: u64, net_bridge_channel_id: u32) -> Self {
        AetherNetDevice {
            iface_id,
            net_bridge_chan_id: net_bridge_channel_id,
            rx_packet_queue: VecDeque::new(),
        }
    }

    pub fn enqueue_rx_packet(&mut self, dma_handle: u64, len: u64) {
        self.rx_packet_queue.push_back((dma_handle, len));
    }
}

impl<'a> Device<'a> for AetherNetDevice {
    type RxToken = PacketRxToken<'a>;
    type TxToken = PacketTxToken<'a>;

    fn capabilities(&self) -> DeviceCapabilities {
        let mut caps = DeviceCapabilities::default();
        caps.max_transmission_unit = 1500;
        caps.max_burst_size = Some(1);
        caps.checksum = Checksum::None; // Checksum offloading not simulated
        caps.medium = smoltcp::phy::Medium::Ethernet;
        caps
    }

    fn receive(&'a mut self, _timestamp: Instant) -> Option<(Self::RxToken, Self::TxToken)> {
        // Consume from the queue of packets pushed by net-bridge
        if let Some((dma_handle, len)) = self.rx_packet_queue.pop_front() {
            if let Ok(buf_ptr) = get_dma_buffer_ptr(dma_handle) {
                let buffer = unsafe { core::slice::from_raw_parts_mut(buf_ptr, len as usize) };
                Some((PacketRxToken { buffer, dma_handle }, PacketTxToken {
                    buffer: &mut [], // dummy buffer for TxToken when not transmitting
                    dma_handle: 0,
                    len: 0,
                    iface_id: self.iface_id,
                    net_bridge_chan_id: self.net_bridge_chan_id // Pass the channel ID
                }))
            } else {
                log(&alloc::format!("AetherNetDevice: Failed to get buffer pointer for RX DMA handle {}. Freeing it.", dma_handle));
                // Free the DMA buffer if ptr is invalid
                if let Err(e) = net_free_buf(dma_handle) { log(&alloc::format!("AetherNetDevice: Failed to free RX DMA buffer (ptr error, queue): {}", e)); }
                None
            }
        } else {
            // No packets from net-bridge in queue
            None
        }
    }

    fn transmit(&'a mut self, _timestamp: Instant) -> Option<Self::TxToken> {
        // Allocate a DMA buffer for outgoing packet
        let dma_handle = match net_alloc_buf(1536) {
            Ok(h) => h,
            Err(e) => { log(&alloc::format!("AetherNetDevice: Failed to alloc TX DMA buffer: {}", e)); return None; }
        };

        if let Ok(buf_ptr) = get_dma_buffer_ptr(dma_handle) {
            let buffer = unsafe { core::slice::from_raw_parts_mut(buf_ptr, 1536) }; // Max MTU
            Some(PacketTxToken { buffer, dma_handle, len: 1536, iface_id: self.iface_id, net_bridge_chan_id: self.net_bridge_chan_id })
        } else {
            log(&alloc::format!("AetherNetDevice: Failed to get buffer pointer for TX DMA handle {}. Freeing it.", dma_handle));
            if let Err(e) = net_free_buf(dma_handle) { log(&alloc::format!("AetherNetDevice: Failed to free TX DMA buffer (ptr error): {}", e)); } // Try to free if ptr couldn't be obtained
            None
        }
    }
}
