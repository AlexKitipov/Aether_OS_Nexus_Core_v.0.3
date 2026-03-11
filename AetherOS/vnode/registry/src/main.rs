#![no_std]
#![no_main]

extern crate alloc;

use core::panic::PanicInfo;
use linked_list_allocator::LockedHeap;
use aetheros_common::ipc::vnode::VNodeChannel;
use aetheros_common::syscall::{syscall3, SYS_LOG};


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
    let _ = syscall3(SYS_LOG, msg.as_ptr() as u64, msg.len() as u64, 0);
}

#[no_mangle]
pub extern "C" fn _start() -> ! {
    init_allocator();
    let mut own_chan = VNodeChannel::new(1);
    log("Registry V-Node starting up...");

    loop {
        if own_chan.recv_blocking().is_err() {
            log("Registry: receive error");
        }
    }
}


#[panic_handler]
fn panic(_info: &PanicInfo) -> ! {
    loop {}
}
