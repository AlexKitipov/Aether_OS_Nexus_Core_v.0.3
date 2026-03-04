// kernel/syscall.rs

#![allow(dead_code)] // Allow dead code for now as not all functions might be used immediately

extern crate alloc;
use alloc::vec::Vec;
use core::str;

use crate::{kprintln, task, ipc, caps, timer};
use crate::arch::x86_64::{irq, dma}; // Use refactored arch modules

// Error codes
pub const E_ACC_DENIED: u64 = 0xFFFFFFFFFFFFFFFE;
pub const E_UNKNOWN_SYSCALL: u64 = 0xFFFFFFFFFFFFFFFF;
pub const E_ERROR: u64 = 1;
pub const SUCCESS: u64 = 0;

// Syscall numbers
pub const SYS_LOG: u64 = 0;
pub const SYS_IPC_SEND: u64 = 1;
pub const SYS_IPC_RECV: u64 = 2;
pub const SYS_BLOCK_ON_CHAN: u64 = 3;
pub const SYS_TIME: u64 = 4;
pub const SYS_IRQ_REGISTER: u64 = 5;
pub const SYS_NET_RX_POLL: u64 = 6;
pub const SYS_NET_ALLOC_BUF: u64 = 7;
pub const SYS_NET_FREE_BUF: u64 = 8;
pub const SYS_NET_TX: u64 = 9;
pub const SYS_IRQ_ACK: u64 = 10;
pub const SYS_GET_DMA_BUF_PTR: u64 = 11;
pub const SYS_SET_DMA_BUF_LEN: u64 = 12;
pub const SYS_IPC_RECV_NONBLOCKING: u64 = 13;

#[no_mangle]
pub extern "C" fn syscall_dispatch(n: u64, a1: u64, a2: u64, a3: u64) -> u64 {
    let current_task = task::get_current_task();

    match n {
        SYS_LOG => {
            if !current_task.capabilities.iter().any(|cap| *cap == caps::Capability::LogWrite) {
                return E_ACC_DENIED;
            }
            let ptr = a1 as *const u8;
            let len = a2 as usize;
            // SAFETY: Caller provides pointer/len pair from V-Node's memory space.
            // The kernel must ensure this is a valid and safe access.
            // For now, we trust the V-Node to provide valid memory.
            let msg = unsafe { core::slice::from_raw_parts(ptr, len) };
            if let Ok(s) = str::from_utf8(msg) {
                kprintln!("[V-Node Log {}] {}", current_task.id, s);
                SUCCESS
            } else {
                kprintln!("[kernel] SYS_LOG: Invalid UTF-8 sequence from task {}.", current_task.id);
                E_ERROR
            }
        }
        SYS_IPC_SEND => {
            if !current_task.capabilities.iter().any(|cap| *cap == caps::Capability::IpcManage) {
                return E_ACC_DENIED;
            }
            let channel_id = a1 as ipc::ChannelId;
            let buf = unsafe { core::slice::from_raw_parts(a2 as *const u8, a3 as usize) };
            if ipc::kernel_send(channel_id, current_task.id, buf).is_ok() {
                SUCCESS
            }
            else {
                E_ERROR
            }
        }
        SYS_IPC_RECV | SYS_IPC_RECV_NONBLOCKING => {
            if !current_task.capabilities.iter().any(|cap| *cap == caps::Capability::IpcManage) {
                return E_ACC_DENIED;
            }
            let channel_id = a1 as ipc::ChannelId;
            let out_ptr = a2 as *mut u8;
            let out_cap = a3 as usize;

            let message = if n == SYS_IPC_RECV {
                // For blocking receive, if no message, block the task
                if !ipc::kernel_peek(channel_id) {
                    task::block_current_on_channel(channel_id);
                    // Scheduler will pick another task. When unblocked, this syscall will be re-entered.
                    return SUCCESS; // Indicate that task is blocked, no data returned yet
                }
                ipc::kernel_recv(channel_id)
            } else { // Non-blocking
                ipc::kernel_recv(channel_id)
            };

            if let Some(data) = message {
                if data.data.len() <= out_cap {
                    // SAFETY: `out_ptr` points to writable buffer of at least `out_cap` from V-Node.
                    // Kernel must ensure this is safe (e.g., page table checks).
                    unsafe {
                        core::ptr::copy_nonoverlapping(data.data.as_ptr(), out_ptr, data.data.len());
                    }
                    data.data.len() as u64
                } else {
                    kprintln!("[kernel] SYS_IPC_RECV: Message too large for V-Node's buffer (task {}).", current_task.id);
                    E_ERROR // Message too large for provided buffer
                }
            } else {
                SUCCESS // No message available or channel empty
            }
        }
        SYS_BLOCK_ON_CHAN => {
            // This syscall is now mostly internal to SYS_IPC_RECV for blocking.
            // If explicitly called, it blocks the current task on a given channel ID.
            task::block_current_on_channel(a1 as u32);
            SUCCESS
        }
        SYS_TIME => {
            if !current_task.capabilities.iter().any(|cap| *cap == caps::Capability::TimeRead) {
                return E_ACC_DENIED;
            }
            timer::get_current_ticks()
        }
        SYS_IRQ_REGISTER => {
            let irq_num = a1 as u8;
            let channel_id = a2 as u32;
            if !current_task.capabilities.iter().any(|cap| *cap == caps::Capability::IrqRegister(irq_num) || *cap == caps::Capability::NetworkAccess) {
                // NetworkAccess is a broad capability that implies IRQ registration for network devices.
                return E_ACC_DENIED;
            }
            irq::register_irq_handler(irq_num, channel_id);
            SUCCESS
        }
        SYS_NET_RX_POLL => {
            // This syscall is highly dependent on specific hardware/driver.
            // For now, it remains a simulation for a network device.
            if !current_task.capabilities.iter().any(|cap| *cap == caps::Capability::NetworkAccess) {
                return E_ACC_DENIED;
            }

            // Simulated ICMP Echo Request packet from previous iteration, moved here.
            let simulated_packet: [u8; 98] = [
                // Ethernet Header (14 bytes)
                0x02, 0x00, 0x00, 0x00, 0x00, 0x01, // Destination MAC (AetherNet's MAC)
                0x00, 0x00, 0x00, 0x00, 0x00, 0x02, // Source MAC (Simulated Sender)
                0x08, 0x00,                         // EtherType: IPv4
                // IPv4 Header (20 bytes)
                0x45, 0x00,                         // Version (4) + IHL (5), DSCP (0)
                0x00, 0x54,                         // Total Length: 84 bytes (20 IP + 8 ICMP + 56 Data)
                0x00, 0x01, 0x00, 0x00,             // Identification, Flags, Fragment Offset
                0x40, 0x01,                         // TTL (64), Protocol (ICMP)
                0x7C, 0x0A,                         // Header Checksum (placeholder, will be calculated by smoltcp)
                0x0A, 0x00, 0x02, 0x01,             // Source IP: 10.0.2.1
                0x0A, 0x00, 0x02, 0x0F,             // Destination IP: 10.0.2.15
                // ICMP Echo Request (8 bytes + 56 bytes data = 64 bytes total for ICMP payload)
                0x08, 0x00,                         // Type (8: Echo Request), Code (0)
                0xF7, 0xFF,                         // Checksum (placeholder, will be calculated by smoltcp)
                0x00, 0x01,                         // ID (1)
                0x00, 0x01,                         // Sequence (1)
                // ICMP Data (56 bytes - 'A' * 56)
                0x41, 0x41, 0x41, 0x41, 0x41, 0x41, 0x41, 0x41,
                0x41, 0x41, 0x41, 0x41, 0x41, 0x41, 0x41, 0x41,
                0x41, 0x41, 0x41, 0x41, 0x41, 0x41, 0x41, 0x41,
                0x41, 0x41, 0x41, 0x41, 0x41, 0x41, 0x41, 0x41,
                0x41, 0x41, 0x41, 0x41, 0x41, 0x41, 0x41, 0x41,
                0x41, 0x41, 0x41, 0x41, 0x41, 0x41, 0x41, 0x41,
                0x41, 0x41, 0x41, 0x41, 0x41, 0x41, 0x41, 0x41,
            ];
            let packet_len = simulated_packet.len();

            let _iface_id = a1; // Not used in current simulation
            let dma_handle = a2;
            let out_cap = a3 as usize;

            if packet_len <= out_cap {
                if let Some(buf_ptr) = dma::get_dma_buffer_ptr(dma_handle) {
                    // SAFETY: Destination pointer comes from managed DMA map and has enough capacity.
                    // We need to ensure buf_ptr is a valid address accessible by the current V-Node.
                    unsafe { core::ptr::copy_nonoverlapping(simulated_packet.as_ptr(), buf_ptr, packet_len); }
                    if dma::set_dma_buffer_len(dma_handle, packet_len).is_ok() {
                        kprintln!("[kernel] SYS_NET_RX_POLL: Simulated packet of {} bytes copied to DMA handle {}.", packet_len, dma_handle);
                        packet_len as u64
                    } else {
                        E_ERROR
                    }
                } else {
                    kprintln!("[kernel] SYS_NET_RX_POLL: DMA buffer pointer not found for handle {}.", dma_handle);
                    E_ERROR
                }
            } else {
                kprintln!("[kernel] SYS_NET_RX_POLL: Simulated packet too large for V-Node's buffer ({} > {}).", packet_len, out_cap);
                E_ERROR
            }
        }
        SYS_NET_ALLOC_BUF => {
            if !current_task.capabilities.iter().any(|cap| *cap == caps::Capability::DmaAlloc || *cap == caps::Capability::NetworkAccess) {
                return E_ACC_DENIED;
            }
            let size = a1 as usize;
            if let Some(handle) = dma::alloc_dma_buffer(size) {
                handle
            }
            else {
                E_ERROR
            }
        }
        SYS_NET_FREE_BUF => {
            if !current_task.capabilities.iter().any(|cap| *cap == caps::Capability::DmaAlloc || *cap == caps::Capability::NetworkAccess) {
                return E_ACC_DENIED;
            }
            dma::free_dma_buffer(a1);
            SUCCESS
        }
        SYS_NET_TX => {
            if !current_task.capabilities.iter().any(|cap| *cap == caps::Capability::NetworkAccess) {
                return E_ACC_DENIED;
            }
            // In a real system, this would queue the DMA buffer for transmission by the NIC driver.
            kprintln!("[kernel] SYS_NET_TX: Queuing packet for TX, handle: {}, len: {}. (Task {})", a2, a3, current_task.id);
            SUCCESS
        }
        SYS_IRQ_ACK => {
            let irq_num = a1 as u8;
            if !current_task.capabilities.iter().any(|cap| *cap == caps::Capability::IrqAck(irq_num) || *cap == caps::Capability::NetworkAccess) {
                return E_ACC_DENIED;
            }
            irq::acknowledge_irq(irq_num);
            SUCCESS
        }
        SYS_GET_DMA_BUF_PTR => {
            if !current_task.capabilities.iter().any(|cap| *cap == caps::Capability::DmaAccess || *cap == caps::Capability::NetworkAccess) {
                 return E_ACC_DENIED;
            }
            if let Some(ptr) = dma::get_dma_buffer_ptr(a1) {
                ptr as u64
            }
            else {
                E_ERROR
            }
        }
        SYS_SET_DMA_BUF_LEN => {
            if !current_task.capabilities.iter().any(|cap| *cap == caps::Capability::DmaAccess || *cap == caps::Capability::NetworkAccess) {
                 return E_ACC_DENIED;
            }
            if dma::set_dma_buffer_len(a1, a2 as usize).is_ok() {
                SUCCESS
            }
            else {
                E_ERROR
            }
        }
        _ => {
            kprintln!("[kernel] syscall: Unknown syscall number {} from task {}.", n, current_task.id);
            E_UNKNOWN_SYSCALL
        }
    }
}
