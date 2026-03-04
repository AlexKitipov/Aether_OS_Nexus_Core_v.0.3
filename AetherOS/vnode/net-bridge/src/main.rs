// vnode/net-bridge/src/main.rs

#![no_std]
#![no_main]

extern crate alloc;

use core::panic::PanicInfo;
use alloc::vec::Vec;
use alloc::format;

use common::ipc::vnode::VNodeChannel;
use common::syscall::{syscall3, SYS_LOG, SYS_IRQ_REGISTER, SYS_NET_RX_POLL, SUCCESS, E_ERROR, SYS_NET_ALLOC_BUF, SYS_NET_FREE_BUF, SYS_NET_TX, SYS_IRQ_ACK, SYS_GET_DMA_BUF_PTR, SYS_SET_DMA_BUF_LEN, SYS_IPC_RECV_NONBLOCKING};
use common::ipc::net_ipc::NetPacketMsg;

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
fn net_alloc_buf(size: usize) -> Result<u64, u64> {
    unsafe {
        let handle = syscall3(SYS_NET_ALLOC_BUF, size as u64, 0, 0);
        if handle == E_ERROR { Err(E_ERROR) } else { Ok(handle) }
    }
}

// Syscall wrapper for SYS_NET_FREE_BUF
fn net_free_buf(handle: u64) -> Result<(), u64> {
    unsafe {
        let res = syscall3(SYS_NET_FREE_BUF, handle, 0, 0);
        if res != SUCCESS { Err(E_ERROR) } else { Ok(()) }
    }
}

// Syscall wrapper for SYS_GET_DMA_BUF_PTR
fn get_dma_buffer_ptr(handle: u64) -> Result<*mut u8, u64> {
    unsafe {
        let ptr = syscall3(SYS_GET_DMA_BUF_PTR, handle, 0, 0);
        if ptr == E_ERROR { Err(E_ERROR) } else { Ok(ptr as *mut u8) }
    }
}

// Syscall wrapper for SYS_SET_DMA_BUF_LEN
fn set_dma_buffer_len(handle: u64, len: usize) -> Result<(), u64> {
    unsafe {
        let res = syscall3(SYS_SET_DMA_BUF_LEN, handle, len as u64, 0);
        if res != SUCCESS { Err(E_ERROR) } else { Ok(()) }
    }
}

// Syscall wrapper for SYS_NET_TX
fn net_tx(iface_id: u64, buf_handle: u64, len: u64) -> Result<(), u64> {
    unsafe {
        let res = syscall3(SYS_NET_TX, iface_id, buf_handle, len);
        if res != SUCCESS { Err(E_ERROR) } else { Ok(()) }
    }
}

#[no_mangle]
pub extern "C" fn _start() -> ! {
    // Net-bridge's own channel ID (to receive IRQ events from kernel)
    // For simplicity, we'll hardcode it to 2. This channel also receives
    // TxPacket messages from the AetherNet service.
    let mut own_chan = VNodeChannel::new(2);

    // Channel to the AetherNet service V-Node (for sending RxPacket and receiving TxPacket messages)
    // Hardcoded to 3 as defined for aethernet-service's client_chan in its main.rs.
    let mut net_stack_chan = VNodeChannel::new(3);

    log("Net-Bridge V-Node starting up...");

    // Dynamically allocate a DMA buffer for receiving network packets.
    // Max Ethernet frame size + some headroom.
    const RX_BUFFER_SIZE: usize = 1536;
    let rx_dma_handle = match net_alloc_buf(RX_BUFFER_SIZE) {
        Ok(handle) => {
            log(&alloc::format!("Net-Bridge: Allocated RX DMA buffer with handle {}.", handle));
            handle
        },
        Err(e) => {
            log(&alloc::format!("Net-Bridge: Failed to allocate RX DMA buffer: {}. Panicking.", e));
            panic!("Failed to allocate RX DMA buffer");
        }
    };

    // Register IRQ 11 (common for VirtIO-Net) for this V-Node's channel (own_chan.id)
    unsafe {
        let res = syscall3(
            SYS_IRQ_REGISTER,
            11 as u64, // IRQ number for VirtIO-Net
            own_chan.id as u64, // Channel ID to route IRQ events
            0 // arg3 is unused
        );
        if res == SUCCESS {
            log("Net-Bridge: Registered IRQ 11 successfully.");
        } else {
            log(&alloc::format!("Net-Bridge: Failed to register IRQ 11: {}. Panicking.", res));
            panic!("Failed to register IRQ 11");
        }
    }

    loop {
        // 1. Check for incoming messages from the AetherNet service (e.g., TxPacket requests)
        if let Ok(Some(net_msg_data)) = own_chan.recv_non_blocking() {
            if let Ok(net_packet_msg) = postcard::from_bytes::<NetPacketMsg>(&net_msg_data) {
                match net_packet_msg {
                    NetPacketMsg::TxPacket { dma_handle, len } => {
                        log(&alloc::format!("Net-Bridge: Received TxPacket from net-stack for handle: {}, len: {}.", dma_handle, len));
                        // Signal the kernel to transmit the packet using the provided DMA buffer.
                        // Assuming interface ID is 0 for now.
                        match net_tx(0, dma_handle, len) {
                            Ok(_) => log(&alloc::format!("Net-Bridge: Successfully queued TX packet for handle {}.", dma_handle)),
                            Err(e) => log(&alloc::format!("Net-Bridge: Failed to queue TX packet for handle {}: {}.", dma_handle, e)),
                        }
                        // After transmission, the DMA buffer should be freed.
                        match net_free_buf(dma_handle) {
                            Ok(_) => log(&alloc::format!("Net-Bridge: Freed TX DMA buffer handle {}.", dma_handle)),
                            Err(e) => log(&alloc::format!("Net-Bridge: Failed to free TX DMA buffer handle {}: {}.", dma_handle, e)),
                        }
                        // Acknowledge back to net-stack that packet was processed (optional, but good practice)
                        net_stack_chan.send(&NetPacketMsg::TxPacketAck).unwrap_or_else(|_| log("Net-Bridge: Failed to send TxPacketAck."));
                    },
                    // We don't expect to receive RxPacket from net-stack on this channel
                    _ => log(&alloc::format!("Net-Bridge: Received unexpected NetPacketMsg on own channel: {:?}.", net_packet_msg)),
                }
            } else {
                log("Net-Bridge: Failed to deserialize NetPacketMsg from net-stack on own channel.");
            }
        }

        // 2. Poll for incoming IRQ events (triggered by hardware, sent by kernel to own_chan)
        // This recv_non_blocking now also catches other IPC messages, so careful distinction is needed.
        if let Ok(Some(irq_event_data)) = own_chan.recv_non_blocking() {
            // In a real scenario, msg_data would contain details about the IRQ event.
            // For now, we assume any message on this channel is an IRQ notification from kernel.
            log("Net-Bridge: Received IRQ event (or other IPC). Polling for packets...");

            // Acknowledge the IRQ to the kernel immediately.
            // The actual IRQ number would be parsed from irq_event_data.
            // For now, assume it's IRQ 11.
            unsafe {
                syscall3(SYS_IRQ_ACK, 11 as u64, 0, 0);
            }

            // Poll for incoming network packets using the pre-allocated DMA buffer.
            let len = unsafe {
                syscall3(
                    SYS_NET_RX_POLL,
                    0 as u64, // Interface ID (from cap, assumed 0 for now)
                    rx_dma_handle as u64,
                    RX_BUFFER_SIZE as u64 // Max buffer length
                )
            };

            if len > SUCCESS {
                log(&alloc::format!("Net-Bridge: Received packet of {} bytes into DMA handle {}.", len, rx_dma_handle));

                // Set the actual length of data received in the DMA buffer.
                if let Err(e) = set_dma_buffer_len(rx_dma_handle, len as usize) {
                    log(&alloc::format!("Net-Bridge: Failed to set RX DMA buffer length: {}.", e));
                    // Handle error, maybe free buffer or retry
                } else {
                    // Send the received packet's DMA handle and length to the AetherNet service.
                    let rx_msg = NetPacketMsg::RxPacket { dma_handle: rx_dma_handle, len };
                    match net_stack_chan.send(&rx_msg) {
                        Ok(_) => log(&alloc::format!("Net-Bridge: Sent RxPacket to net-stack for handle {}.", rx_dma_handle)),
                        Err(_) => log(&alloc::format!("Net-Bridge: Failed to send RxPacket to net-stack for handle {}.", rx_dma_handle)),
                    }
                    // The AetherNet service is now responsible for processing and eventually freeing this buffer.
                    // We don't free rx_dma_handle here, as it's passed with ownership semantics to net-stack.
                    // A new RX DMA buffer should be allocated for the next reception, or this V-Node could manage a pool.
                    // For simplicity, we assume net-stack frees it and we'll re-use the conceptual handle (which is problematic for real system).

                    // For this simple example, since we 'transfer ownership' of the buffer to net-stack,
                    // we conceptually need a new one for the next RX_POLL. Reallocating for simplicity.
                    // NOTE: This re-allocation approach is inefficient. A ring buffer or pool of DMA buffers is preferred.
                    // For now, we'll keep it simple to match the current stub nature.

                }

            } else if len == SUCCESS {
                log("Net-Bridge: SYS_NET_RX_POLL returned no packets (expected if IRQ was spurious or handled).");
            } else if len == E_ERROR {
                log("Net-Bridge: SYS_NET_RX_POLL returned an error.");
            } else {
                log(&alloc::format!("Net-Bridge: SYS_NET_RX_POLL returned unknown error code: {}.", len));
            }
        }

        // No blocking call here to allow checking both incoming IPC types.
        // A real driver might use `syscall_wait_for_multiple_channels` if available.
        // For now, this busy-loop can be relieved by kernel scheduling.
    }
}

#[panic_handler]
pub extern "C" fn panic(info: &PanicInfo) -> ! {
    log(&alloc::format!("Net-Bridge V-Node panicked! Info: {:?}.", info));
    loop {}
}
