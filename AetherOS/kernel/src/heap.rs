// kernel/src/heap.rs

#![allow(dead_code)] // Allow dead code for now as not all functions might be used immediately

use crate::kprintln;
use linked_list_allocator::LockedHeap;
use x86_64::VirtAddr;

pub const HEAP_START: u64 = crate::arch::x86_64::paging::KERNEL_VIRT_OFFSET + 0x0200_0000;
pub const HEAP_SIZE: usize = 1024 * 1024;
pub const HEAP_GUARD_PAGES: u64 = 1;
pub const HEAP_PAGE_SIZE: u64 = 4096;
pub const HEAP_MAPPED_START: u64 = HEAP_START + HEAP_GUARD_PAGES * HEAP_PAGE_SIZE;

/// A dummy global allocator that panics on allocation.
/// This will be replaced by our `LockedHeap` once memory mapping is ready.
#[global_allocator]
static ALLOCATOR: LockedHeap = LockedHeap::empty();

/// Initializes the heap allocator.
///
/// This function is unsafe because the caller must guarantee that the given
/// `heap_start` and `heap_size` define a valid, unused region of memory
/// that is mapped correctly to physical frames.
pub unsafe fn init(heap_start: VirtAddr, heap_size: usize) {
    ALLOCATOR.lock().init(heap_start.as_mut_ptr(), heap_size);
    kprintln!("[kernel] heap: Initialized heap at {:#x} with size {} bytes.", heap_start.as_u64(), heap_size);
}




/// Initializes a small early heap region used by kernel allocations.
pub fn init_heap() {
    // SAFETY: The mapped heap starts after one guard page to help catch overflows.
    unsafe { init(VirtAddr::new(HEAP_MAPPED_START), HEAP_SIZE) };
}
