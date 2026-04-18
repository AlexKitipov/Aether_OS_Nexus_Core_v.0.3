#![no_std]

use common::syscall::{
    E_ERROR,
    SUCCESS,
    SYS_CREATE_CHANNEL,
    SYS_EXIT,
    SYS_IPC_RECV,
    SYS_IPC_RECV_NONBLOCKING,
    SYS_IPC_SEND,
    SYS_LOG,
    SYS_TIME,
};

extern "C" {
    fn log_from_vnode(message_ptr: *const u8, message_len: usize);
}

#[no_mangle]
pub extern "C" fn syscall_handler(
    syscall_num: u64,
    arg1: u64,
    arg2: u64,
    arg3: u64,
    _arg4: u64,
) -> u64 {
    match syscall_num {
        SYS_LOG => {
            unsafe { log_from_vnode(arg1 as *const u8, arg2 as usize) }
            SUCCESS
        }
        SYS_EXIT => loop {
            x86_64::instructions::hlt();
        },
        SYS_TIME => {
            crate::task::schedule();
            SUCCESS
        }
        SYS_CREATE_CHANNEL => crate::ipc::create_channel() as u64,
        SYS_IPC_SEND => {
            let channel_id = arg1 as u32;
            let message_ptr = arg2 as *const u8;
            let message_len = arg3 as usize;
            let sender_id = crate::task::get_current_task().id;
            let bytes = unsafe { core::slice::from_raw_parts(message_ptr, message_len) };

            match crate::ipc::kernel_send(channel_id, sender_id, bytes) {
                Ok(_) => SUCCESS,
                Err(_) => E_ERROR,
            }
        }
        SYS_IPC_RECV | SYS_IPC_RECV_NONBLOCKING => {
            let channel_id = arg1 as u32;
            let buffer_ptr = arg2 as *mut u8;
            let buffer_len = arg3 as usize;

            if syscall_num == SYS_IPC_RECV && !crate::ipc::kernel_peek(channel_id) {
                crate::task::block_current_on_channel(channel_id);
                return SUCCESS;
            }

            match crate::ipc::kernel_recv(channel_id) {
                Some(message) => {
                    let Some(bytes) = message.as_inline() else {
                        return E_ERROR;
                    };
                    if bytes.len() > buffer_len {
                        return E_ERROR;
                    }
                    unsafe { core::ptr::copy_nonoverlapping(bytes.as_ptr(), buffer_ptr, bytes.len()) }
                    bytes.len() as u64
                }
                None => SUCCESS,
            }
        }
        _ => E_ERROR,
    }
}
