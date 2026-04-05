// vnode/net-stack/src/aethernet_device.rs

#![no_std]

extern crate alloc;

use alloc::collections::VecDeque; // Added for VecDeque
use smoltcp::phy::{Device, RxToken, TxToken, Checksum, DeviceCapabilities};
use smoltcp::time::Instant;
use smoltcp::wire::{EthernetAddress, HardwareAddress};

use crate::ipc::vnode::VNodeChannel;
use crate::syscall::{syscall3, SYS_LOG, SUCCESS, E_ERROR, SYS_NET_ALLOC_BUF, SYS_NET_FREE_BUF, SYS_GET_DMA_BUF_PTR, SYS_SET_DMA_BUF_LEN, SYS_NET_TX};
use crate::ipc::net_ipc::NetPacketMsg;

const TX_BUFFER_SIZE: usize = 1536;
const MAX_DMA_VIEW_LEN: usize = TX_BUFFER_SIZE;

/// Validates DMA pointer/length pairs before converting into `&mut [u8]`.
///
/// # Safety contract
/// The kernel-side DMA allocator guarantees that successful handles map to a
/// unique writable region until the handle is freed. This helper validates the
/// parts that cannot be proven from Rust types alone:
/// - pointer must be non-null (including zero-length views)
/// - length must fit in `usize` and remain within the agreed DMA window
///
/// Violations are treated as interface contract errors and surfaced to logs so
/// interface-driven debugging can distinguish bad packets from parser failures.
fn dma_view_mut<'a>(ptr: *mut u8, len_u64: u64) -> Result<&'a mut [u8], &'static str> {
    if ptr.is_null() {
        return Err("DMA pointer was null");
    }

    let len = usize::try_from(len_u64).map_err(|_| "DMA length overflow")?;
    if len > MAX_DMA_VIEW_LEN {
        return Err("DMA length exceeded negotiated MTU window");
    }

    // SAFETY: `ptr` is non-null and comes from kernel DMA allocation. Length is
    // bounded to the negotiated buffer window and converted losslessly to usize.
    Ok(unsafe { core::slice::from_raw_parts_mut(ptr, len) })
}


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

// Syscall wrapper for SYS_NET_ALLOC_BUF
pub fn net_alloc_buf(size: usize) -> Result<u64, u64> {
    unsafe {
        let handle = syscall3(SYS_NET_ALLOC_BUF, size as u64, 0, 0);
        if handle == E_ERROR { Err(E_ERROR) } else { Ok(handle) }
    }
}

// Syscall wrapper for SYS_NET_FREE_BUF
pub fn net_free_buf(handle: u64) -> Result<(), u64> {
    unsafe {
        let res = syscall3(SYS_NET_FREE_BUF, handle, 0, 0);
        if res != SUCCESS { Err(E_ERROR) } else { Ok(()) }
    }
}

// Syscall wrapper for SYS_GET_DMA_BUF_PTR
pub fn get_dma_buffer_ptr(handle: u64) -> Result<*mut u8, u64> {
    unsafe {
        let ptr = syscall3(SYS_GET_DMA_BUF_PTR, handle, 0, 0);
        if ptr == E_ERROR { Err(E_ERROR) } else { Ok(ptr as *mut u8) }
    }
}

// Syscall wrapper for SYS_SET_DMA_BUF_LEN
pub fn set_dma_buffer_len(handle: u64, len: usize) -> Result<(), u64> {
    unsafe {
        let res = syscall3(SYS_SET_DMA_BUF_LEN, handle, len as u64, 0);
        if res != SUCCESS { Err(E_ERROR) } else { Ok(()) }
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
            log(&alloc::format!("AetherNetDevice: Failed to free RX DMA buffer (handle {}): {:?}", self.dma_handle, e));
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
        let result = f(self.buffer); // smoltcp fills the buffer

        // Update the actual length of data written by smoltcp
        self.len = self.buffer.len();
        if let Err(e) = set_dma_buffer_len(self.dma_handle, self.len) {
            log(&alloc::format!("AetherNetDevice: Failed to set TX DMA buffer length (handle {}): {:?}", self.dma_handle, e));
            // Attempt to free the buffer even on error
            if let Err(e) = net_free_buf(self.dma_handle) { log(&alloc::format!("AetherNetDevice: Failed to free TX DMA buffer after set_len error (handle {}): {:?}", self.dma_handle, e)); }
            return result;
        }

        // Send the filled buffer's DMA handle and length to net-bridge for transmission
        let mut net_bridge_chan = VNodeChannel::new(self.net_bridge_chan_id);
        let msg = NetPacketMsg::TxPacket { dma_handle: self.dma_handle, len: self.len as u64 };

        net_bridge_chan.send(&msg).unwrap_or_else(|_| log(&alloc::format!("AetherNetDevice: Failed to send TxPacket to net-bridge for handle: {}.", self.dma_handle)));

        // The net-bridge V-Node is now responsible for freeing the DMA buffer after transmission
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
                let buffer = match dma_view_mut(buf_ptr, len) {
                    Ok(buffer) => buffer,
                    Err(e) => {
                        log(&alloc::format!(
                            "AetherNetDevice: Rejecting RX DMA handle {} due to invalid view: {}.",
                            dma_handle,
                            e
                        ));
                        if let Err(free_err) = net_free_buf(dma_handle) {
                            log(&alloc::format!(
                                "AetherNetDevice: Failed to free RX DMA buffer after invalid view (handle {}): {:?}",
                                dma_handle,
                                free_err
                            ));
                        }
                        return None;
                    }
                };
                Some((
                    PacketRxToken { buffer, dma_handle }, 
                    // Dummy TxToken for receive path, as receive doesn't directly transmit
                    PacketTxToken {
                        buffer: &mut [],
                        dma_handle: 0,
                        len: 0,
                        iface_id: self.iface_id,
                        net_bridge_chan_id: self.net_bridge_chan_id,
                    }
                ))
            } else {
                log(&alloc::format!("AetherNetDevice: Failed to get buffer pointer for RX DMA handle {}. Freeing it.", dma_handle));
                // Free the DMA buffer if ptr is invalid, as it's unusable.
                if let Err(e) = net_free_buf(dma_handle) { 
                    log(&alloc::format!("AetherNetDevice: Failed to free RX DMA buffer (ptr error, queue) {}: {:?}", dma_handle, e)); 
                }
                None
            }
        } else {
            // No packets from net-bridge in queue
            None
        }
    }

    fn transmit(&'a mut self, _timestamp: Instant) -> Option<Self::TxToken> {
        // Allocate a DMA buffer for outgoing packet
        // The size is typically the MTU + Ethernet header size
        let dma_handle = match net_alloc_buf(TX_BUFFER_SIZE) {
            Ok(h) => h,
            Err(e) => { log(&alloc::format!("AetherNetDevice: Failed to alloc TX DMA buffer: {:?}", e)); return None; }
        };

        if let Ok(buf_ptr) = get_dma_buffer_ptr(dma_handle) {
            let buffer = match dma_view_mut(buf_ptr, TX_BUFFER_SIZE as u64) {
                Ok(buffer) => buffer,
                Err(e) => {
                    log(&alloc::format!(
                        "AetherNetDevice: Rejecting TX DMA handle {} due to invalid view: {}.",
                        dma_handle,
                        e
                    ));
                    if let Err(free_err) = net_free_buf(dma_handle) {
                        log(&alloc::format!(
                            "AetherNetDevice: Failed to free TX DMA buffer after invalid view (handle {}): {:?}",
                            dma_handle,
                            free_err
                        ));
                    }
                    return None;
                }
            };
            Some(PacketTxToken { buffer, dma_handle, len: 0, iface_id: self.iface_id, net_bridge_chan_id: self.net_bridge_chan_id })
        } else {
            log(&alloc::format!("AetherNetDevice: Failed to get buffer pointer for TX DMA handle {}. Freeing it.", dma_handle));
            // If we can't get a pointer, the buffer is unusable, so free it.
            if let Err(e) = net_free_buf(dma_handle) { 
                log(&alloc::format!("AetherNetDevice: Failed to free TX DMA buffer after ptr error (handle {}): {:?}", dma_handle, e)); 
            }
            None
        }
    }
}
