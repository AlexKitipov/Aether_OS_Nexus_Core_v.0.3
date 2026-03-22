// kernel/syscall.rs

#![allow(dead_code)] // Allow dead code for now as not all functions might be used immediately

extern crate alloc;
use core::str;
use alloc::vec;
use aetheros_common::syscall::{
    E_ACC_DENIED,
    E_ERROR,
    E_UNKNOWN_SYSCALL,
    SUCCESS,
    SYS_BLOCK_ON_CHAN,
    SYS_CAP_GRANT,
    SYS_GET_DMA_BUF_PTR,
    SYS_IPC_RECV,
    SYS_IPC_RECV_NONBLOCKING,
    SYS_IPC_SEND,
    SYS_IRQ_ACK,
    SYS_IRQ_REGISTER,
    SYS_LOG,
    SYS_NET_ALLOC_BUF,
    SYS_NET_FREE_BUF,
    SYS_NET_RX_POLL,
    SYS_NET_TX,
    SYS_SET_DMA_BUF_LEN,
    SYS_TIME,
};

use crate::{kprintln, task, ipc, caps, timer};
use crate::arch::x86_64::{dma, irq}; // Use refactored arch modules
use crate::usercopy::copy_from_user;
const SYS_LOG_MAX_LEN: usize = 1024;
const SYS_IPC_MAX_LEN: usize = 4096;

const CAP_LOG_WRITE: u64 = 0;
const CAP_TIME_READ: u64 = 1;
const CAP_NETWORK_ACCESS: u64 = 2;
const CAP_STORAGE_ACCESS: u64 = 3;
const CAP_IRQ_REGISTER: u64 = 4;
const CAP_DMA_ALLOC: u64 = 5;
const CAP_DMA_ACCESS: u64 = 6;
const CAP_IRQ_ACK: u64 = 7;
const CAP_IPC_MANAGE: u64 = 8;

/// Initialize the syscall subsystem.
///
/// At the moment this enables only the high-level dispatcher surface.
/// The architecture-specific `SYSCALL/SYSRET` entry trampoline can be wired
/// in a later phase under `arch/x86_64`.
pub fn init() {
    kprintln!("[kernel] syscall: dispatcher initialized.");
}

fn decode_capability(kind: u64, arg: u64) -> Option<caps::Capability> {
    match kind {
        CAP_LOG_WRITE => Some(caps::Capability::LogWrite),
        CAP_TIME_READ => Some(caps::Capability::TimeRead),
        CAP_NETWORK_ACCESS => Some(caps::Capability::NetworkAccess),
        CAP_STORAGE_ACCESS => Some(caps::Capability::StorageAccess),
        CAP_IRQ_REGISTER => Some(caps::Capability::IrqRegister(arg as u8)),
        CAP_DMA_ALLOC => Some(caps::Capability::DmaAlloc),
        CAP_DMA_ACCESS => Some(caps::Capability::DmaAccess),
        CAP_IRQ_ACK => Some(caps::Capability::IrqAck(arg as u8)),
        CAP_IPC_MANAGE => Some(caps::Capability::IpcManage),
        _ => None,
    }
}

#[no_mangle]
pub extern "C" fn syscall_dispatch(n: u64, a1: u64, a2: u64, a3: u64) -> u64 {
    let current_task = task::get_current_task();

    match n {
        SYS_LOG => {
            if !caps::Capability::LogWrite.check(current_task.id) {
                return E_ACC_DENIED;
            }
            let ptr = a1 as *const u8;
            let len = (a2 as usize).min(SYS_LOG_MAX_LEN);
            let mut msg = vec![0u8; len];
            if let Err(err) = copy_from_user(&mut msg, ptr) {
                kprintln!(
                    "[kernel] SYS_LOG: rejected user buffer from task {}: {}.",
                    current_task.id,
                    err
                );
                return E_ACC_DENIED;
            }
            if let Ok(s) = str::from_utf8(&msg) {
                kprintln!("[V-Node Log {}] {}", current_task.id, s);
                SUCCESS
            } else {
                kprintln!("[kernel] SYS_LOG: Invalid UTF-8 sequence from task {}.", current_task.id);
                E_ERROR
            }
        }
        SYS_IPC_SEND => {
            if !caps::Capability::IpcManage.check(current_task.id) {
                return E_ACC_DENIED;
            }
            let channel_id = a1 as ipc::ChannelId;
            let msg_len = a3 as usize;
            if msg_len > SYS_IPC_MAX_LEN {
                return E_ERROR;
            }

            let mut msg = vec![0u8; msg_len];
            if let Err(err) = copy_from_user(&mut msg, a2 as *const u8) {
                kprintln!(
                    "[kernel] SYS_IPC_SEND: rejected user buffer from task {}: {}.",
                    current_task.id,
                    err
                );
                return E_ACC_DENIED;
            }

            if ipc::mailbox::send(channel_id, current_task.id as u32, &msg).is_ok() {
                SUCCESS
            }
            else {
                E_ERROR
            }
        }
        SYS_IPC_RECV | SYS_IPC_RECV_NONBLOCKING => {
            if !caps::Capability::IpcManage.check(current_task.id) {
                return E_ACC_DENIED;
            }
            let channel_id = a1 as ipc::ChannelId;
            let out_ptr = a2 as *mut u8;
            let out_cap = a3 as usize;
            let blocking = n == SYS_IPC_RECV;

            match ipc::mailbox::recv_message(channel_id, out_ptr, out_cap, blocking) {
                Ok(len) => len as u64,
                Err(_err) => E_ERROR,
            }
        }
        SYS_BLOCK_ON_CHAN => {
            // This syscall is now mostly internal to SYS_IPC_RECV for blocking.
            // If explicitly called, it blocks the current task on a given channel ID.
            task::block_current_on_channel(a1 as u32);
            SUCCESS
        }
        SYS_TIME => {
            if !caps::Capability::TimeRead.check(current_task.id) {
                return E_ACC_DENIED;
            }
            timer::get_current_ticks()
        }
        SYS_IRQ_REGISTER => {
            let irq_num = a1 as u8;
            let channel_id = a2 as u32;
            if !(caps::Capability::IrqRegister(irq_num).check(current_task.id) || caps::Capability::NetworkAccess.check(current_task.id)) {
                // NetworkAccess is a broad capability that implies IRQ registration for network devices.
                return E_ACC_DENIED;
            }

            if irq_num == 1 {
                // IRQ1 is routed through the dedicated PS/2 keyboard handler,
                // which reads port 0x60 and IPC-forwards scancodes.
                crate::interrupts::keyboard::register_channel(channel_id);
            } else {
                irq::register_irq_handler(irq_num, channel_id);
            }
            SUCCESS
        }
        SYS_NET_RX_POLL => {
            // This syscall is highly dependent on specific hardware/driver.
            // For now, it remains a simulation for a network device.
            if !caps::Capability::NetworkAccess.check(current_task.id) {
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
            if !(caps::Capability::DmaAlloc.check(current_task.id) || caps::Capability::NetworkAccess.check(current_task.id)) {
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
            if !(caps::Capability::DmaAlloc.check(current_task.id) || caps::Capability::NetworkAccess.check(current_task.id)) {
                return E_ACC_DENIED;
            }
            dma::free_dma_buffer(a1);
            SUCCESS
        }
        SYS_NET_TX => {
            if !caps::Capability::NetworkAccess.check(current_task.id) {
                return E_ACC_DENIED;
            }
            // In a real system, this would queue the DMA buffer for transmission by the NIC driver.
            kprintln!("[kernel] SYS_NET_TX: Queuing packet for TX, handle: {}, len: {}. (Task {})", a2, a3, current_task.id);
            SUCCESS
        }
        SYS_IRQ_ACK => {
            let irq_num = a1 as u8;
            if !(caps::Capability::IrqAck(irq_num).check(current_task.id) || caps::Capability::NetworkAccess.check(current_task.id)) {
                return E_ACC_DENIED;
            }
            irq::acknowledge_irq(irq_num);
            SUCCESS
        }
        SYS_GET_DMA_BUF_PTR => {
            if !(caps::Capability::DmaAccess.check(current_task.id) || caps::Capability::NetworkAccess.check(current_task.id)) {
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
            if !(caps::Capability::DmaAccess.check(current_task.id) || caps::Capability::NetworkAccess.check(current_task.id)) {
                 return E_ACC_DENIED;
            }
            if dma::set_dma_buffer_len(a1, a2 as usize).is_ok() {
                SUCCESS
            }
            else {
                E_ERROR
            }
        }
        SYS_CAP_GRANT => {
            // Delegation is a privileged operation: the caller must be able to manage IPC/cap routing.
            if !caps::Capability::IpcManage.check(current_task.id) {
                return E_ACC_DENIED;
            }

            let target_task_id = a1;
            let cap_kind = a2;
            let cap_arg = a3;
            let Some(cap) = decode_capability(cap_kind, cap_arg) else {
                kprintln!(
                    "[kernel] SYS_CAP_GRANT: Invalid capability kind {} from task {}.",
                    cap_kind,
                    current_task.id
                );
                return E_ERROR;
            };

            if caps::transfer_capability(current_task.id, target_task_id, cap) {
                SUCCESS
            } else {
                kprintln!(
                    "[kernel] SYS_CAP_GRANT: Delegation of {:?} from task {} to task {} denied.",
                    cap,
                    current_task.id,
                    target_task_id
                );
                E_ACC_DENIED
            }
        }
        _ => {
            kprintln!("[kernel] syscall: Unknown syscall number {} from task {}.", n, current_task.id);
            E_UNKNOWN_SYSCALL
        }
    }
}
