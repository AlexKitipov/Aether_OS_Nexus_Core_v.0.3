#![no_std]
#![no_main]

extern crate alloc;

use alloc::vec;
use core::panic::PanicInfo;

use common::ipc::vfs_ipc::VfsRequest;
use common::syscall;
use linked_list_allocator::LockedHeap;

const VNODE_HEAP_SIZE: usize = 64 * 1024;
static mut VNODE_HEAP: [u8; VNODE_HEAP_SIZE] = [0; VNODE_HEAP_SIZE];

#[global_allocator]
static GLOBAL_ALLOCATOR: LockedHeap = LockedHeap::empty();


fn init_allocator() {
    unsafe {
        GLOBAL_ALLOCATOR.lock().init(VNODE_HEAP.as_mut_ptr(), VNODE_HEAP_SIZE);
    }
}

fn log(msg: &str) {
    let _ = syscall::syscall3(
        syscall::SYS_LOG,
        msg.as_ptr() as u64,
        msg.len() as u64,
        0,
    );
}

fn send_vfs_read_request(channel_id: u64, path: &str) {
    let req = VfsRequest::Read {
        fd: 0,
        len: 256,
        offset: 0,
    };
    if let Ok(payload) = postcard::to_allocvec(&req) {
        let _ = syscall::syscall3(
            syscall::SYS_IPC_SEND,
            channel_id,
            payload.as_ptr() as u64,
            payload.len() as u64,
        );
    }

    let stat_req = VfsRequest::Stat { path: path.into() };
    if let Ok(payload) = postcard::to_allocvec(&stat_req) {
        let _ = syscall::syscall3(
            syscall::SYS_IPC_SEND,
            channel_id,
            payload.as_ptr() as u64,
            payload.len() as u64,
        );
    }
}

#[no_mangle]
pub extern "C" fn _start() -> ! {
    init_allocator();

    log("[test-vnode] booting user-mode sample");
    let ticks = syscall::syscall3(syscall::SYS_TIME, 0, 0, 0);
    let tick_log = if ticks == 0 {
        "[test-vnode] time syscall returned zero"
    } else {
        "[test-vnode] time syscall reachable"
    };
    log(tick_log);

    // Conceptual demo: channel 1 would be the AetherFS/VFS vnode endpoint.
    send_vfs_read_request(1, "/initrd/manifest.json");
    let _scratch = vec![0u8; 128];

    loop { }
}

#[panic_handler]
fn panic(_info: &PanicInfo) -> ! {
    loop { }
}
