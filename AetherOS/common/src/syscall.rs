
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

/// System call return codes.
pub const SUCCESS: u64 = 0;
pub const E_ERROR: u64 = 1;
pub const E_UNKNOWN_SYSCALL: u64 = 0xFFFFFFFFFFFFFFFF;
pub const E_ACC_DENIED: u64 = 0xFFFFFFFFFFFFFFFE;

/// Performs a system call with three arguments.
#[must_use]
#[inline(always)]
pub fn syscall3(syscall_num: u64, arg1: u64, arg2: u64, arg3: u64) -> u64 {
    let ret: u64;
    unsafe {
        core::arch::asm!(
            "int $0x80",
            in("rax") syscall_num,
            in("rdi") arg1,
            in("rsi") arg2,
            in("rdx") arg3,
            lateout("rax") ret,
            options(nostack, preserves_flags)
        );
    }
    ret
}

/// Performs a system call with four arguments.
#[must_use]
#[inline(always)]
pub fn syscall4(syscall_num: u64, arg1: u64, arg2: u64, arg3: u64, arg4: u64) -> u64 {
    let ret: u64;
    unsafe {
        core::arch::asm!(
            "int $0x80",
            in("rax") syscall_num,
            in("rdi") arg1,
            in("rsi") arg2,
            in("rdx") arg3,
            in("r8") arg4,
            lateout("rax") ret,
            options(nostack, preserves_flags)
        );
    }
    ret
}
