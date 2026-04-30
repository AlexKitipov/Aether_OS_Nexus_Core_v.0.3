#![no_std]
#![no_main]
#![feature(custom_test_frameworks)]
#![feature(alloc_error_handler)] // Explicitly needed for test binary
#![test_runner(crate::test_runner)]
#![reexport_test_harness_main = "test_main"]

extern crate alloc;
use alloc::alloc::{GlobalAlloc, Layout};
use alloc::string::String;
use alloc::vec;

use core::panic::PanicInfo;
use linked_list_allocator::LockedHeap;

// Import necessary modules from the library crate
// Removed `extern crate adi_sandbox;` as this is implicitly linked when running tests for the crate
use crate::model::{Capability, DriverModel};
use crate::governor::Governor;
use crate::runtime::Runtime;

// --- Global Allocator and Error Handler for tests binary ---
pub const HEAP_SIZE: usize = 4 * 1024; // 4 KiB
static mut HEAP_MEMORY: [u8; HEAP_SIZE] = [0; HEAP_SIZE];

#[global_allocator]
static ALLOCATOR: LockedHeap = LockedHeap::empty();

#[alloc_error_handler]
fn alloc_error_handler(layout: Layout) -> ! {
    panic!("Allocation error in test: {:?}", layout)
}
// --- End Global Allocator for tests binary ---

#[no_mangle]
pub extern "C" fn _start() -> ! {
    // Initialize the heap allocator for the test binary
    let heap_start: *mut u8 = 0x_4444_4444_4444_4444 as *mut u8;
    unsafe { ALLOCATOR.lock().init(heap_start, HEAP_SIZE); }

    test_main();
    loop {}
}

#[panic_handler]
fn panic(_info: &PanicInfo) -> ! {
    // A basic panic handler for test environment
    // print! is from the dummy macro below
    println!("Panic: {}", _info);
    loop {}
}

pub fn test_runner(tests: &[&dyn Fn()]) {
    println!("Running {} tests", tests.len());
    for test in tests {
        test();
    }
}

/// A dummy println! macro for the no_std environment
/// In a real test environment, this would output to console/serial
#[macro_export]
macro_rules! println {
    ($($arg:tt)*) => {};
}


#[test_case]
fn test_validate_driver_valid_model() {
    // Create a Governor with allowed capabilities
    let allowed_caps = vec![Capability::HardwareAccess, Capability::NetworkAccess];
    let governor = Governor::new(allowed_caps);

    // Create a DriverModel that requires an allowed capability
    let driver_model = DriverModel::new(
        1,
        String::from("ValidDriver"),
        String::from("1.0.0"),
        vec![Capability::HardwareAccess],
        vec![]
    );

    // Create a Runtime instance with the Governor
    let runtime = Runtime::new(1024 * 1024, 1000, governor);

    // Assert that validation passes for the valid driver model
    assert!(runtime.validate_driver(&driver_model).is_ok());
}

#[test_case]
fn test_validate_driver_invalid_model() {
    // Create a Governor with allowed capabilities (e.g., only HardwareAccess)
    let allowed_caps = vec![Capability::HardwareAccess];
    let governor = Governor::new(allowed_caps);

    // Create a DriverModel that requires a disallowed capability (e.g., NetworkAccess)
    let driver_model = DriverModel.new(
        2,
        String::from("InvalidDriver"),
        String::from("1.0.0"),
        vec![Capability::NetworkAccess],
        vec![]
    );

    // Create a Runtime instance with the Governor
    let runtime = Runtime::new(1024 * 1024, 1000, governor);

    // Assert that validation fails for the invalid driver model
    assert!(runtime.validate_driver(&driver_model).is_err());
}
