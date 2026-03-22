#![no_std]
#![no_main]

extern crate alloc;

use alloc::format;
use core::panic::PanicInfo;
use linked_list_allocator::LockedHeap;

use common::ipc::vnode::VNodeChannel;
use common::syscall::{syscall3, SYS_IPC_RECV, SYS_LOG, SUCCESS};

const VNODE_HEAP_SIZE: usize = 64 * 1024;
static mut VNODE_HEAP: [u8; VNODE_HEAP_SIZE] = [0; VNODE_HEAP_SIZE];

const KEYBOARD_IRQ_CHANNEL_ID: u32 = 4;

#[global_allocator]
static GLOBAL_ALLOCATOR: LockedHeap = LockedHeap::empty();

fn init_allocator() {
    unsafe {
        GLOBAL_ALLOCATOR
            .lock()
            .init(VNODE_HEAP.as_mut_ptr(), VNODE_HEAP_SIZE);
    }
}

fn log(msg: &str) {
    unsafe {
        let _ = syscall3(SYS_LOG, msg.as_ptr() as u64, msg.len() as u64, 0);
    }
}

#[no_mangle]
pub extern "C" fn _start() -> ! {
    init_allocator();
    let irq_chan = VNodeChannel::new(KEYBOARD_IRQ_CHANNEL_ID);

    log("Keyboard V-Node started.");

    let mut raw = [0u8; 16];
    loop {
        let recv_len = unsafe {
            syscall3(
                SYS_IPC_RECV,
                irq_chan.id as u64,
                raw.as_mut_ptr() as u64,
                raw.len() as u64,
            )
        };

        if recv_len == 0 {
            continue;
        }

        if recv_len == SUCCESS {
            continue;
        }

        let scancode = raw[0];
        log(&format!("keyboard: scancode=0x{:02x}", scancode));
    }
}

#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    log(&format!("Keyboard V-Node panic: {:?}", info));
    loop {}
}
