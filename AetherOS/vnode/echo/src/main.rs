#![no_std]
#![no_main]

extern crate alloc;

use core::panic::PanicInfo;

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


#[no_mangle]
pub extern "C" fn _start() -> ! {
    init_allocator();
    loop { }
}

#[panic_handler]
fn panic(_info: &PanicInfo) -> ! {
    loop { }
}
