#![no_std] // We don't want the Rust standard library in our common library.

/// System call numbers.
pub const SYS_LOG: u64 = 1;
pub const SYS_EXIT: u64 = 2;
pub const SYS_TIME: u64 = 3;
pub const SYS_IPC_SEND: u64 = 4;
pub const SYS_IPC_RECV: u64 = 5;
pub const SYS_IPC_RECV_NONBLOCKING: u64 = 6;
pub const SYS_CREATE_CHANNEL: u64 = 7;

/// System call return codes.
pub const SUCCESS: u64 = 0;
pub const E_ERROR: u64 = 1;
pub const E_UNKNOWN_SYSCALL: u64 = 0xFFFFFFFFFFFFFFFF;
pub const E_ACC_DENIED: u64 = 0xFFFFFFFFFFFFFFFE;

/// Performs a system call with three arguments.
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
