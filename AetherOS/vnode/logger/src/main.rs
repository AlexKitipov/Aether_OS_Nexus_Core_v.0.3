#![no_std]
#![no_main]

extern crate alloc;

use alloc::format;
use core::panic::PanicInfo;

use common::ipc::logger_ipc::{LogLevel, LoggerRequest, LoggerResponse};
use common::ipc::vnode::VNodeChannel;
use common::IpcSend;
use common::syscall::{syscall_log, syscall3, SYS_TIME};
use linked_list_allocator::LockedHeap;

const VNODE_HEAP_SIZE: usize = 64 * 1024;
static mut VNODE_HEAP: [u8; VNODE_HEAP_SIZE] = [0; VNODE_HEAP_SIZE];

/// Channel used by clients (e.g. shell) to send logger IPC requests.
const LOGGER_CLIENT_CHANNEL_ID: u32 = 9;

#[global_allocator]
static GLOBAL_ALLOCATOR: LockedHeap = LockedHeap::empty();

#[alloc_error_handler]
fn alloc_error_handler(_layout: core::alloc::Layout) -> ! {
    loop {}
}

fn init_allocator() {
    unsafe {
        GLOBAL_ALLOCATOR
            .lock()
            .init(VNODE_HEAP.as_mut_ptr(), VNODE_HEAP_SIZE);
    }
}

fn level_label(level: &LogLevel) -> &'static str {
    match level {
        LogLevel::Trace => "TRACE",
        LogLevel::Debug => "DEBUG",
        LogLevel::Info => "INFO",
        LogLevel::Warn => "WARN",
        LogLevel::Error => "ERROR",
        LogLevel::Fatal => "FATAL",
    }
}

fn emit_kernel_log(level: &LogLevel, message: &str) {
    let line = format!("[logger:{}] {}", level_label(level), message);
    let _ = syscall_log(&line);
}

fn run_loop() -> ! {
    let mut client_chan = VNodeChannel::new(LOGGER_CLIENT_CHANNEL_ID);
    let _ = syscall_log("Logger V-Node: online");

    loop {
        if let Ok(Some(req_data)) = client_chan.recv_non_blocking() {
            let response = match postcard::from_bytes::<LoggerRequest>(&req_data) {
                Ok(LoggerRequest::Log { message, level }) => {
                    emit_kernel_log(&level, &message);
                    LoggerResponse::Success
                }
                Err(_) => {
                    let _ = syscall_log("Logger V-Node: failed to deserialize LoggerRequest");
                    LoggerResponse::Error("invalid logger request payload".into())
                }
            };

            let _ = client_chan.send(&response);
        }

        let _ = syscall3(SYS_TIME, 0, 0, 0);
    }
}

#[no_mangle]
pub extern "C" fn _start() -> ! {
    init_allocator();
    run_loop();
}

#[panic_handler]
fn panic(_info: &PanicInfo) -> ! {
    let _ = syscall_log("Logger V-Node panicked");
    loop {}
}
