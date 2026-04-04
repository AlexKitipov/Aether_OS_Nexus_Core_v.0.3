
/// System call numbers.
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
pub const SYS_CAP_GRANT: u64 = 14;
pub const SYS_UI_CALL: u64 = 15;
pub const SYS_SWARM_CALL: u64 = 16;
pub const SYS_AI_CALL: u64 = 17;
pub const SYS_VFS_CALL: u64 = 18;
pub const SYS_UDP_SEND: u64 = 19;
pub const SYS_UDP_RECV: u64 = 20;


/// Syscall ABI version used by user-space V-Nodes and the kernel dispatcher.
///
/// Bump this constant whenever syscall numbers or argument contracts change.
pub const SYSCALL_ABI_VERSION: u64 = 2;
/// Maximum number of syscall arguments supported by ABI v2.
///
/// x86_64 register mapping for `syscall`:
/// - `rax`: syscall number
/// - `rdi`, `rsi`, `rdx`, `r10`, `r8`, `r9`: args 1..=6
pub const SYSCALL_ABI_MAX_ARGS: u64 = 6;

/// System call return codes.
pub const SUCCESS: u64 = 0;
pub const E_ERROR: u64 = 1;
pub const E_UNKNOWN_SYSCALL: u64 = 0xFFFFFFFFFFFFFFFF;
pub const E_ACC_DENIED: u64 = 0xFFFFFFFFFFFFFFFE;

const _: () = {
    assert!(SYS_LOG == 0);
    assert!(SYS_IPC_SEND == 1);
    assert!(SYS_IPC_RECV == 2);
    assert!(SYS_TIME == 4);
    assert!(SYS_IRQ_REGISTER == 5);
    assert!(SYS_IRQ_ACK == 10);
    assert!(SYS_IPC_RECV_NONBLOCKING == 13);
    assert!(SYS_CAP_GRANT == 14);
    assert!(SYS_UI_CALL == 15);
    assert!(SYS_SWARM_CALL == 16);
    assert!(SYS_AI_CALL == 17);
    assert!(SYS_VFS_CALL == 18);
    assert!(SYS_UDP_SEND == 19);
    assert!(SYS_UDP_RECV == 20);
};

/// ABI-safe user buffer descriptor passed over the syscall boundary.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct UserBuf {
    pub ptr: u64,
    pub len: u64,
}

impl UserBuf {
    #[must_use]
    pub fn from_slice(slice: &[u8]) -> Self {
        Self {
            ptr: slice.as_ptr() as u64,
            len: slice.len() as u64,
        }
    }
}


/// Performs a system call with no arguments.
#[must_use]
#[inline(always)]
pub fn syscall0(syscall_num: u64) -> u64 {
    syscall3(syscall_num, 0, 0, 0)
}

/// Performs a system call with one argument.
#[must_use]
#[inline(always)]
pub fn syscall1(syscall_num: u64, arg1: u64) -> u64 {
    syscall3(syscall_num, arg1, 0, 0)
}

/// Performs a system call with two arguments.
#[must_use]
#[inline(always)]
pub fn syscall2(syscall_num: u64, arg1: u64, arg2: u64) -> u64 {
    syscall3(syscall_num, arg1, arg2, 0)
}

/// Performs a system call with three arguments.
///
/// x86_64 ABI used by the kernel entry glue:
/// - rax: syscall number
/// - rdi, rsi, rdx: args 1..=3
/// - return value in rax
#[must_use]
#[inline(always)]
pub fn syscall3(syscall_num: u64, arg1: u64, arg2: u64, arg3: u64) -> u64 {
    let ret: u64;
    // SAFETY: This emits a single `syscall` instruction using the x86_64 Linux-style
    // register ABI expected by the AetherOS kernel entry glue:
    // rax=syscall number, rdi/rsi/rdx=args 1..3, rcx/r11 clobbered by hardware.
    // No stack memory is touched by the asm block itself (`nostack`).
    unsafe {
        core::arch::asm!(
            "syscall",
            in("rax") syscall_num,
            in("rdi") arg1,
            in("rsi") arg2,
            in("rdx") arg3,
            lateout("rax") ret,
            lateout("rcx") _,
            lateout("r11") _,
            options(nostack)
        );
    }
    ret
}

/// Performs a system call with four arguments.
///
/// x86_64 syscall ABI places the 4th argument in r10.
#[must_use]
#[inline(always)]
pub fn syscall4(syscall_num: u64, arg1: u64, arg2: u64, arg3: u64, arg4: u64) -> u64 {
    let ret: u64;
    // SAFETY: Same syscall ABI as `syscall3`, with arg4 in `r10`.
    unsafe {
        core::arch::asm!(
            "syscall",
            in("rax") syscall_num,
            in("rdi") arg1,
            in("rsi") arg2,
            in("rdx") arg3,
            in("r10") arg4,
            lateout("rax") ret,
            lateout("rcx") _,
            lateout("r11") _,
            options(nostack)
        );
    }
    ret
}


/// Performs a system call with five arguments.
///
/// x86_64 syscall ABI places args 4 and 5 in r10 and r8.
#[must_use]
#[inline(always)]
pub fn syscall5(syscall_num: u64, arg1: u64, arg2: u64, arg3: u64, arg4: u64, arg5: u64) -> u64 {
    let ret: u64;
    // SAFETY: Same syscall ABI as `syscall3`, with arg4 in `r10` and arg5 in `r8`.
    unsafe {
        core::arch::asm!(
            "syscall",
            in("rax") syscall_num,
            in("rdi") arg1,
            in("rsi") arg2,
            in("rdx") arg3,
            in("r10") arg4,
            in("r8") arg5,
            lateout("rax") ret,
            lateout("rcx") _,
            lateout("r11") _,
            options(nostack)
        );
    }
    ret
}

/// Performs a system call with six arguments.
///
/// x86_64 syscall ABI places args 4, 5, 6 in r10, r8, r9.
#[must_use]
#[inline(always)]
pub fn syscall6(
    syscall_num: u64,
    arg1: u64,
    arg2: u64,
    arg3: u64,
    arg4: u64,
    arg5: u64,
    arg6: u64,
) -> u64 {
    let ret: u64;
    // SAFETY: Same syscall ABI as `syscall3`, with args 4..6 in r10/r8/r9.
    unsafe {
        core::arch::asm!(
            "syscall",
            in("rax") syscall_num,
            in("rdi") arg1,
            in("rsi") arg2,
            in("rdx") arg3,
            in("r10") arg4,
            in("r8") arg5,
            in("r9") arg6,
            lateout("rax") ret,
            lateout("rcx") _,
            lateout("r11") _,
            options(nostack)
        );
    }
    ret
}

/// Writes a UTF-8 log message through the kernel log syscall.
///
/// This helper centralizes the ABI contract for `SYS_LOG`:
/// - `arg1`: user pointer to message bytes
/// - `arg2`: message length
/// - `arg3`: reserved (must be zero for now)
#[must_use]
#[inline(always)]
pub fn syscall_log(message: &str) -> u64 {
    syscall3(SYS_LOG, message.as_ptr() as u64, message.len() as u64, 0)
}

/// Sends an opaque request payload to a UI service endpoint over the syscall ABI.
///
/// This is an ABI-level alias for channel-based IPC send and is intended to make
/// domain intent explicit in user-mode call sites.
#[must_use]
#[inline(always)]
pub fn syscall_ui_call(service_channel: u64, payload: &[u8]) -> u64 {
    syscall3(
        SYS_UI_CALL,
        service_channel,
        payload.as_ptr() as u64,
        payload.len() as u64,
    )
}

/// Sends an opaque request payload to a Swarm service endpoint over the syscall ABI.
#[must_use]
#[inline(always)]
pub fn syscall_swarm_call(service_channel: u64, payload: &[u8]) -> u64 {
    syscall3(
        SYS_SWARM_CALL,
        service_channel,
        payload.as_ptr() as u64,
        payload.len() as u64,
    )
}

/// Sends an opaque request payload to an AI runtime endpoint over the syscall ABI.
#[must_use]
#[inline(always)]
pub fn syscall_ai_call(service_channel: u64, payload: &[u8]) -> u64 {
    syscall3(
        SYS_AI_CALL,
        service_channel,
        payload.as_ptr() as u64,
        payload.len() as u64,
    )
}

/// Sends an opaque request payload to a VFS endpoint over the syscall ABI.
#[must_use]
#[inline(always)]
pub fn syscall_vfs_call(service_channel: u64, payload: &[u8]) -> u64 {
    syscall3(
        SYS_VFS_CALL,
        service_channel,
        payload.as_ptr() as u64,
        payload.len() as u64,
    )
}


#[must_use]
#[inline(always)]
pub fn syscall_udp_send(local_port: u16, remote_ip: [u8; 4], remote_port: u16, payload: &[u8]) -> u64 {
    syscall6(
        SYS_UDP_SEND,
        local_port as u64,
        u32::from_be_bytes(remote_ip) as u64,
        remote_port as u64,
        payload.as_ptr() as u64,
        payload.len() as u64,
        0,
    )
}

#[must_use]
#[inline(always)]
pub fn syscall_udp_recv(local_port: u16, out: &mut [u8]) -> u64 {
    syscall4(
        SYS_UDP_RECV,
        local_port as u64,
        out.as_mut_ptr() as u64,
        out.len() as u64,
        0,
    )
}
