// kernel/src/heap.rs

#![allow(dead_code)] // Allow dead code for now as not all functions might be used immediately

use linked_list_allocator::LockedHeap;
use x86_64::{VirtAddr, PhysAddr};
use x86_64::structures::paging::{Page, PageTableFlags, Size4KiB, Mapper, FrameAllocator};
use crate::kprintln;
use crate::memory::page_allocator::PageAllocator;

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


