// vnode/net-bridge/src/main.rs

#![cfg_attr(target_os = "none", no_std)]
#![cfg_attr(target_os = "none", no_main)]
#![cfg_attr(not(target_os = "none"), allow(dead_code, unused_imports, unused_variables, unused_unsafe))]

extern crate alloc;

#[cfg(target_os = "none")]
use core::panic::PanicInfo;
use spin::Mutex;

use common::ipc::net_ipc::NetPacketMsg;
use common::ipc::vnode::VNodeChannel;
use common::syscall::{
    syscall3,
    SYS_LOG,
    SYS_IRQ_REGISTER,
    SYS_NET_RX_POLL,
    SUCCESS,
    E_ERROR,
    SYS_NET_ALLOC_BUF,
    SYS_NET_FREE_BUF,
    SYS_NET_TX,
    SYS_IRQ_ACK,
    SYS_GET_DMA_BUF_PTR,
    SYS_SET_DMA_BUF_LEN,
};
use common::vnode_heap::VNodeHeap;

// Temporary log function for V-Nodes

const VNODE_HEAP_SIZE: usize = 64 * 1024;
static mut VNODE_HEAP: [u8; VNODE_HEAP_SIZE] = [0; VNODE_HEAP_SIZE];

const RX_BUFFER_SIZE: usize = 2048;
const RX_POOL_SIZE: usize = 64;

struct RxDmaHandlePool {
    handles: [u64; RX_POOL_SIZE],
    len: usize,
}

impl RxDmaHandlePool {
    const fn new() -> Self {
        Self {
            handles: [0; RX_POOL_SIZE],
            len: 0,
        }
    }

    fn push(&mut self, handle: u64) {
        if self.len < RX_POOL_SIZE {
            self.handles[self.len] = handle;
            self.len += 1;
        } else {
            let _ = net_free_buf(handle);
        }
    }

    fn pop(&mut self) -> Option<u64> {
        if self.len == 0 {
            None
        } else {
            self.len -= 1;
            Some(self.handles[self.len])
        }
    }
}

static RX_DMA_POOL: Mutex<RxDmaHandlePool> = Mutex::new(RxDmaHandlePool::new());

#[global_allocator]
static GLOBAL_ALLOCATOR: VNodeHeap = VNodeHeap::new();

fn init_allocator() {
    unsafe {
        let heap_ptr = core::ptr::addr_of_mut!(VNODE_HEAP);
        GLOBAL_ALLOCATOR.init_buffer(&mut (*heap_ptr)[..]);
    }
}

fn init_rx_pool() {
    let mut pool = RX_DMA_POOL.lock();
    for _ in 0..RX_POOL_SIZE {
        match net_alloc_buf(RX_BUFFER_SIZE) {
            Ok(handle) => {
                pool.push(handle);
            }
            Err(e) => {
                log(&alloc::format!("Net-Bridge: init_rx_pool failed to allocate DMA buffer: {}", e));
                break;
            }
        }
    }
}

fn get_rx_dma_handle() -> Result<u64, u64> {
    if let Some(handle) = RX_DMA_POOL.lock().pop() {
        Ok(handle)
    } else {
        net_alloc_buf(RX_BUFFER_SIZE)
    }
}

fn return_rx_dma_handle(handle: u64) {
    RX_DMA_POOL.lock().push(handle);
}

fn log(msg: &str) {
    let res = syscall3(
        SYS_LOG,
        msg.as_ptr() as u64,
        msg.len() as u64,
        0 // arg3 is unused for SYS_LOG
    );
    if res != SUCCESS { /* Handle log error, maybe panic or fall back */ }
}

// Syscall wrapper for SYS_NET_ALLOC_BUF
fn net_alloc_buf(size: usize) -> Result<u64, u64> {
    let handle = syscall3(SYS_NET_ALLOC_BUF, size as u64, 0, 0);
    if handle == E_ERROR { Err(E_ERROR) } else { Ok(handle) }
}

// Syscall wrapper for SYS_NET_FREE_BUF
fn net_free_buf(handle: u64) -> Result<(), u64> {
    let res = syscall3(SYS_NET_FREE_BUF, handle, 0, 0);
    if res != SUCCESS { Err(E_ERROR) } else { Ok(()) }
}

// Syscall wrapper for SYS_GET_DMA_BUF_PTR
fn get_dma_buffer_ptr(handle: u64) -> Result<*mut u8, u64> {
    let ptr = syscall3(SYS_GET_DMA_BUF_PTR, handle, 0, 0);
    if ptr == E_ERROR { Err(E_ERROR) } else { Ok(ptr as *mut u8) }
}

// Syscall wrapper for SYS_SET_DMA_BUF_LEN
fn set_dma_buffer_len(handle: u64, len: usize) -> Result<(), u64> {
    let res = syscall3(SYS_SET_DMA_BUF_LEN, handle, len as u64, 0);
    if res != SUCCESS { Err(E_ERROR) } else { Ok(()) }
}

// Syscall wrapper for SYS_NET_TX
fn net_tx(iface_id: u64, buf_handle: u64, len: u64) -> Result<(), u64> {
    let res = syscall3(SYS_NET_TX, iface_id, buf_handle, len);
    if res != SUCCESS { Err(E_ERROR) } else { Ok(()) }
}

#[cfg(target_os = "none")]
#[no_mangle]
pub extern "C" fn _start() -> ! {
    init_allocator();
    // Net-bridge's own channel ID (to receive IRQ events from kernel)
    // For simplicity, we'll hardcode it to 2. This channel also receives
    // TxPacket messages from the AetherNet service.
    let mut own_chan = VNodeChannel::new(2);

    // Channel to the AetherNet service V-Node (for sending RxPacket and receiving TxPacket messages)
    // Hardcoded to 3 as defined for aethernet-service's client_chan in its main.rs.
    let mut net_stack_chan = VNodeChannel::new(3);

    log("Net-Bridge V-Node starting up...");

    init_rx_pool();

    let mut rx_dma_handle = match get_rx_dma_handle() {
        Ok(handle) => {
            log(&alloc::format!("Net-Bridge: Acquired RX DMA handle {} from pool.", handle));
            handle
        }
        Err(e) => {
            log(&alloc::format!("Net-Bridge: Failed to acquire RX DMA handle: {}. Panicking.", e));
            panic!("Failed to acquire RX DMA handle");
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

                if let Err(e) = set_dma_buffer_len(rx_dma_handle, len as usize) {
                    log(&alloc::format!("Net-Bridge: Failed to set RX DMA buffer length: {}. Replacing RX buffer handle.", e));
                    rx_dma_handle = get_rx_dma_handle().unwrap_or_else(|e| {
                        log(&alloc::format!("Net-Bridge: Failed to replace RX DMA handle: {}. Panicking.", e));
                        panic!("Failed to replace RX DMA handle");
                    });
                } else {
                    let rx_msg = NetPacketMsg::RxPacket { dma_handle: rx_dma_handle, len };
                    match net_stack_chan.send(&rx_msg) {
                        Ok(_) => {
                            log(&alloc::format!("Net-Bridge: Sent RxPacket to net-stack for handle {}.", rx_dma_handle));
                            rx_dma_handle = get_rx_dma_handle().unwrap_or_else(|e| {
                                log(&alloc::format!("Net-Bridge: Failed to acquire next RX DMA handle: {}. Panicking.", e));
                                panic!("Failed to acquire next RX DMA handle");
                            });
                        }
                        Err(_) => {
                            log(&alloc::format!("Net-Bridge: Failed to send RxPacket to net-stack for handle {}. Returning handle to pool.", rx_dma_handle));
                            return_rx_dma_handle(rx_dma_handle);
                            rx_dma_handle = get_rx_dma_handle().unwrap_or_else(|e| {
                                log(&alloc::format!("Net-Bridge: Failed to reacquire RX DMA handle after send failure: {}. Panicking.", e));
                                panic!("Failed to reacquire RX DMA handle");
                            });
                        }
                    }
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

#[cfg(target_os = "none")]
#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    log(&alloc::format!("Net-Bridge V-Node panicked! Info: {:?}.", info));
    loop {}
}


#[cfg(not(target_os = "none"))]
fn main() {
    println!("net-bridge host stub: target_os != none");
}
