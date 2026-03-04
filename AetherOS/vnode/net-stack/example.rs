// vnode/net-stack/example.rs
// This is an example Rust file for the net-stack V-Node.
// You can replace this content with your actual network stack code.

#![no_std]
#![no_main]

use core::panic::PanicInfo;

#[no_mangle]
pub extern "C" fn _start() -> ! {
    // Your network stack initialization and main loop will go here.
    // For now, it's just a placeholder.
    loop {}
}

#[panic_handler]
fn panic(_info: &PanicInfo) -> ! {
    loop {}
}
