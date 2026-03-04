#![no_std] // We don't link the Rust standard library

use common::syscall::{
    E_ERROR, SUCCESS, SYS_CREATE_CHANNEL, SYS_EXIT, SYS_IPC_RECV, SYS_IPC_RECV_NONBLOCKING,
    SYS_IPC_SEND, SYS_LOG, SYS_TIME,
};

extern "C" {
    // Function to handle logging from V-Nodes
    fn log_from_vnode(message_ptr: *const u8, message_len: usize);
}

#[no_mangle]
pub extern "C" fn syscall_handler(
    syscall_num: u64,
    arg1: u64,
    arg2: u64,
    arg3: u64,
    arg4: u64,
) -> u64 {
    match syscall_num {
        SYS_LOG => {
            unsafe {
                // arg1 is message_ptr, arg2 is message_len
                log_from_vnode(arg1 as *const u8, arg2 as usize);
            }
            SUCCESS
        }
        SYS_EXIT => {
            crate::task::scheduler::terminate_current_task();
            SUCCESS
        }
        SYS_TIME => {
            // TODO: Implement proper yielding to scheduler
            crate::task::schedule();
            SUCCESS
        }
        SYS_CREATE_CHANNEL => {
            let channel_id = crate::ipc::mailbox::create_channel();
            channel_id as u64
        }
        SYS_IPC_SEND => {
            let channel_id = arg1 as u32;
            let message_ptr = arg2 as *const u8;
            let message_len = arg3 as usize;
            match crate::ipc::mailbox::send_message(channel_id, message_ptr, message_len) {
                Ok(_) => SUCCESS,
                Err(_) => E_ERROR,
            }
        }
        SYS_IPC_RECV => {
            let channel_id = arg1 as u32;
            let buffer_ptr = arg2 as *mut u8;
            let buffer_len = arg3 as usize;
            match crate::ipc::mailbox::recv_message(channel_id, buffer_ptr, buffer_len, true) {
                Ok(len) => len as u64,
                Err(_) => E_ERROR,
            }
        }
        SYS_IPC_RECV_NONBLOCKING => {
            let channel_id = arg1 as u32;
            let buffer_ptr = arg2 as *mut u8;
            let buffer_len = arg3 as usize;
            match crate::ipc::mailbox::recv_message(channel_id, buffer_ptr, buffer_len, false) {
                Ok(len) => len as u64,
                Err(_) => E_ERROR,
            }
        }
        _ => E_ERROR, // Unknown syscall
    }
}
