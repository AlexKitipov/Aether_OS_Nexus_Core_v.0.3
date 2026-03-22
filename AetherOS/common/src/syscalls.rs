//! Legacy syscall module compatibility shim.
//!
//! This crate historically contained a kernel-side syscall dispatcher copy here.
//! That implementation was stale and unsafe for userspace pointers.
//! Keep this file as a thin compatibility layer so any out-of-tree imports of
//! `common::syscalls::*` continue to build while using the canonical x86_64
//! `SYSCALL` user ABI wrappers from `common::syscall`.

#![allow(dead_code)]

pub use crate::syscall::{
    E_ACC_DENIED,
    E_ERROR,
    SYSCALL_ABI_VERSION,
    E_UNKNOWN_SYSCALL,
    SUCCESS,
    SYS_AI_CALL,
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
    UserBuf,
    syscall_ai_call,
    syscall0,
    syscall1,
    syscall2,
    syscall3,
    syscall4,
    syscall5,
    syscall6,
    syscall_swarm_call,
    syscall_ui_call,
    syscall_vfs_call,
};
