// kernel/syscall.rs

#![allow(dead_code)] // Allow dead code for now as not all functions might be used immediately

extern crate alloc;
use aetheros_common::syscall::{
    E_ACC_DENIED,
    E_ERROR,
    E_UNKNOWN_SYSCALL,
    SUCCESS,
    SYSCALL_ABI_VERSION,
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
    SYS_SWARM_CALL,
    SYS_TIME,
    SYS_UI_CALL,
    SYS_VFS_CALL,
    SYS_UDP_SEND,
    SYS_UDP_RECV,
    SYS_AI_CALL,
};

use crate::{caps, ipc, irq, kprintln, task, timer};
use crate::arch::x86_64::dma; // Use refactored arch modules
use crate::usercopy::{copy_from_user, copy_to_user, copy_utf8_from_user};
use aetheros_common::channel::well_known;
use core::cmp::min;
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
    kprintln!(
        "[kernel] syscall: dispatcher initialized (ABI v{}).",
        SYSCALL_ABI_VERSION
    );
}

fn read_user_bytes(
    ptr: *const u8,
    len: usize,
    max_len: usize,
) -> Result<alloc::vec::Vec<u8>, &'static str> {
    let len = len.min(max_len);
    let mut buf = alloc::vec![0u8; len];
    copy_from_user(&mut buf, ptr)?;
    Ok(buf)
}

fn read_user_utf8(
    ptr: *const u8,
    len: usize,
    max_len: usize,
) -> Result<alloc::string::String, &'static str> {
    copy_utf8_from_user(ptr, len, max_len)
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

fn syscall_ipc_send(current_task_id: u64, channel_id: ipc::ChannelId, ptr: *const u8, msg_len: usize) -> u64 {
    let msg = match read_user_bytes(ptr, msg_len, SYS_IPC_MAX_LEN) {
        Ok(msg) => msg,
        Err(err) => {
            kprintln!(
                "[kernel] SYS_IPC_SEND: rejected user buffer from task {}: {}.",
                current_task_id,
                err
            );
            return E_ACC_DENIED;
        }
    };

    if ipc::mailbox::send(channel_id, current_task_id as u32, &msg).is_ok() {
        SUCCESS
    } else {
        E_ERROR
    }
}

#[derive(Debug, Clone, Copy)]
pub enum VNodeIoSyscall {
    Read(crate::device::DeviceId, usize),
    Write(crate::device::DeviceId, usize),
}

fn execute_vnode_io(
    vnode: &crate::device::VNode,
    syscall: VNodeIoSyscall,
    read_buf: Option<&mut [u8]>,
    write_buf: Option<&[u8]>,
) -> crate::device::IoResult<usize> {
    match syscall {
        VNodeIoSyscall::Read(dev, len) => {
            if !vnode.has_cap(dev, crate::device::Right::Read) {
                return Err(crate::device::IoError::PermissionDenied);
            }
            crate::device::with_manager(|manager| {
                let io = manager
                    .get_io(dev)
                    .ok_or(crate::device::IoError::DeviceNotFound)?;
                let buf = read_buf.ok_or(crate::device::IoError::Fault)?;
                let read_len = core::cmp::min(len, buf.len());
                io.read(&mut buf[..read_len])
            })
            .ok_or(crate::device::IoError::DeviceNotFound)?
        }
        VNodeIoSyscall::Write(dev, len) => {
            if !vnode.has_cap(dev, crate::device::Right::Write) {
                return Err(crate::device::IoError::PermissionDenied);
            }
            crate::device::with_manager(|manager| {
                let io = manager
                    .get_io(dev)
                    .ok_or(crate::device::IoError::DeviceNotFound)?;
                let buf = write_buf.ok_or(crate::device::IoError::Fault)?;
                io.write(&buf[..core::cmp::min(len, buf.len())])
            })
            .ok_or(crate::device::IoError::DeviceNotFound)?
        }
    }
}

/// Kernel-safe read API for V-Nodes.
pub fn vnode_read(dev: crate::device::DeviceId, buf: &mut [u8]) -> crate::device::IoResult<usize> {
    let task_id = task::scheduler::get_current_task_id();
    let vnode = crate::device::vnode_caps_from_task(task_id);
    execute_vnode_io(&vnode, VNodeIoSyscall::Read(dev, buf.len()), Some(buf), None)
}

/// Kernel-safe write API for V-Nodes.
pub fn vnode_write(dev: crate::device::DeviceId, buf: &[u8]) -> crate::device::IoResult<usize> {
    let task_id = task::scheduler::get_current_task_id();
    let vnode = crate::device::vnode_caps_from_task(task_id);
    execute_vnode_io(&vnode, VNodeIoSyscall::Write(dev, buf.len()), None, Some(buf))
}

#[derive(Clone, Copy)]
struct SyscallArgs {
    a1: u64,
    a2: u64,
    a3: u64,
    a4: u64,
    a5: u64,
    a6: u64,
}

fn syscall_dispatch_inner(n: u64, args: SyscallArgs) -> u64 {
    let current_task = task::get_current_task();

    match n {
        SYS_LOG => {
            if !caps::Capability::LogWrite.check(current_task.id) {
                return E_ACC_DENIED;
            }
            let ptr = args.a1 as *const u8;
            let len = args.a2 as usize;
            let msg = match read_user_utf8(ptr, len, SYS_LOG_MAX_LEN) {
                Ok(msg) => msg,
                Err(err) => {
                    kprintln!(
                        "[kernel] SYS_LOG: rejected user buffer from task {}: {}.",
                        current_task.id,
                        err
                    );
                    return E_ACC_DENIED;
                }
            };
            kprintln!("[V-Node Log {}] {}", current_task.id, msg);
            SUCCESS
        }
        SYS_IPC_SEND => {
            if !caps::Capability::IpcManage.check(current_task.id) {
                return E_ACC_DENIED;
            }
            syscall_ipc_send(
                current_task.id,
                args.a1 as ipc::ChannelId,
                args.a2 as *const u8,
                args.a3 as usize,
            )
        }
        SYS_UI_CALL | SYS_SWARM_CALL | SYS_AI_CALL | SYS_VFS_CALL => {
            if !caps::Capability::IpcManage.check(current_task.id) {
                return E_ACC_DENIED;
            }

            let service_channel = args.a1 as ipc::ChannelId;
            let route_allowed = match n {
                SYS_UI_CALL => well_known::is_ui(service_channel),
                SYS_SWARM_CALL => well_known::is_swarm(service_channel),
                SYS_AI_CALL => well_known::is_ai(service_channel),
                SYS_VFS_CALL => well_known::is_vfs(service_channel),
                _ => false,
            };

            if !route_allowed {
                kprintln!(
                    "[kernel] domain syscall {} denied for task {} on unknown channel {}.",
                    n,
                    current_task.id,
                    service_channel
                );
                return E_ERROR;
            }

            // ABI v2 domain-specific calls are routed through the same secure mailbox path,
            // but must target a known service endpoint for their declared domain.
            syscall_ipc_send(
                current_task.id,
                service_channel,
                args.a2 as *const u8,
                args.a3 as usize,
            )
        }
        SYS_IPC_RECV | SYS_IPC_RECV_NONBLOCKING => {
            if !caps::Capability::IpcManage.check(current_task.id) {
                return E_ACC_DENIED;
            }
            let channel_id = args.a1 as ipc::ChannelId;
            let out_ptr = args.a2 as *mut u8;
            let out_cap = args.a3 as usize;
            let blocking = n == SYS_IPC_RECV;

            match ipc::mailbox::recv_message(channel_id, out_ptr, out_cap, blocking) {
                Ok(len) => len as u64,
                Err(_err) => E_ERROR,
            }
        }
        SYS_BLOCK_ON_CHAN => {
            // This syscall is now mostly internal to SYS_IPC_RECV for blocking.
            // If explicitly called, it blocks the current task on a given channel ID.
            task::block_current_on_channel(args.a1 as u32);
            SUCCESS
        }
        SYS_TIME => {
            if !caps::Capability::TimeRead.check(current_task.id) {
                return E_ACC_DENIED;
            }
            timer::get_current_ticks()
        }
        SYS_IRQ_REGISTER => {
            let irq_num = args.a1 as u8;
            let channel_id = args.a2 as u32;
            if !(caps::Capability::IrqRegister(irq_num).check(current_task.id) || caps::Capability::NetworkAccess.check(current_task.id)) {
                // NetworkAccess is a broad capability that implies IRQ registration for network devices.
                return E_ACC_DENIED;
            }

            irq::register_irq_handler(irq_num, channel_id);
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

            let _iface_id = args.a1; // Not used in current simulation
            let dma_handle = args.a2;
            let out_cap = args.a3 as usize;

            if packet_len <= out_cap {
                let Some(buf_capacity) = dma::get_dma_buffer_capacity(dma_handle) else {
                    kprintln!(
                        "[kernel] SYS_NET_RX_POLL: DMA buffer capacity lookup failed for handle {}.",
                        dma_handle
                    );
                    return E_ERROR;
                };

                if packet_len > buf_capacity {
                    kprintln!(
                        "[kernel] SYS_NET_RX_POLL: packet {} exceeds DMA capacity {} for handle {}.",
                        packet_len,
                        buf_capacity,
                        dma_handle
                    );
                    return E_ERROR;
                }

                if dma::write_to_buffer(dma_handle, &simulated_packet, 0).is_err() {
                    kprintln!(
                        "[kernel] SYS_NET_RX_POLL: failed writing packet into DMA handle {}.",
                        dma_handle
                    );
                    return E_ERROR;
                }

                if dma::set_dma_buffer_len(dma_handle, packet_len).is_ok() {
                    kprintln!(
                        "[kernel] SYS_NET_RX_POLL: Simulated packet of {} bytes copied to DMA handle {}.",
                        packet_len,
                        dma_handle
                    );
                    packet_len as u64
                } else {
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
            let size = args.a1 as usize;
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
            dma::free_dma_buffer(args.a1);
            SUCCESS
        }
        SYS_NET_TX => {
            if !caps::Capability::NetworkAccess.check(current_task.id) {
                return E_ACC_DENIED;
            }
            // In a real system, this would queue the DMA buffer for transmission by the NIC driver.
            kprintln!("[kernel] SYS_NET_TX: Queuing packet for TX, handle: {}, len: {}. (Task {})", args.a2, args.a3, current_task.id);
            SUCCESS
        }

        SYS_UDP_SEND => {
            if !caps::Capability::NetworkAccess.check(current_task.id) {
                return E_ACC_DENIED;
            }

            let local_port = args.a1 as u16;
            let ip = (args.a2 as u32).to_be_bytes();
            let remote_ip = core::net::Ipv4Addr::new(ip[0], ip[1], ip[2], ip[3]);
            let remote_port = args.a3 as u16;

            let payload = match read_user_bytes(args.a4 as *const u8, args.a5 as usize, SYS_IPC_MAX_LEN) {
                Ok(data) => data,
                Err(_) => return E_ACC_DENIED,
            };

            match crate::network::with_stack(|stack| {
                stack.udp_send(current_task.id, local_port, remote_ip, remote_port, &payload)
            }) {
                Some(Ok(sent)) => sent as u64,
                _ => E_ERROR,
            }
        }
        SYS_UDP_RECV => {
            if !caps::Capability::NetworkAccess.check(current_task.id) {
                return E_ACC_DENIED;
            }

            let local_port = args.a1 as u16;
            let out_ptr = args.a2 as *mut u8;
            let out_cap = min(args.a3 as usize, SYS_IPC_MAX_LEN);
            let mut buffer = alloc::vec![0u8; out_cap];

            let recv_len = match crate::network::with_stack(|stack| {
                stack.udp_recv(current_task.id, local_port, &mut buffer)
            }) {
                Some(Ok(n)) => n,
                _ => return E_ERROR,
            };

            // SAFETY: user pointer validity is checked via copy helper semantics.
            if copy_to_user(out_ptr, &buffer[..recv_len]).is_err() {
                return E_ACC_DENIED;
            }
            recv_len as u64
        }

        SYS_IRQ_ACK => {
            let irq_num = args.a1 as u8;
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
            if let Some(ptr) = dma::get_dma_buffer_ptr(args.a1) {
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
            if dma::set_dma_buffer_len(args.a1, args.a2 as usize).is_ok() {
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

            let target_task_id = args.a1;
            let cap_kind = args.a2;
            let cap_arg = args.a3;
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

/// Compatibility dispatcher used by older entry glue that forwards only 3 args.
#[no_mangle]
pub extern "C" fn syscall_dispatch(n: u64, a1: u64, a2: u64, a3: u64) -> u64 {
    syscall_dispatch_inner(
        n,
        SyscallArgs {
            a1,
            a2,
            a3,
            a4: 0,
            a5: 0,
            a6: 0,
        },
    )
}

/// Primary ABI v2 syscall dispatcher with the full six x86_64 syscall arguments.
#[no_mangle]
pub extern "C" fn syscall_dispatch6(
    n: u64,
    a1: u64,
    a2: u64,
    a3: u64,
    a4: u64,
    a5: u64,
    a6: u64,
) -> u64 {
    syscall_dispatch_inner(
        n,
        SyscallArgs {
            a1,
            a2,
            a3,
            a4: a4,
            a5: a5,
            a6: a6,
        },
    )
}
