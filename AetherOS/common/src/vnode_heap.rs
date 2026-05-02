use core::alloc::{GlobalAlloc, Layout};
use linked_list_allocator::LockedHeap;

/// Helper for user-mode VNode heap initialization.
///
/// This wrapper centralizes the `LockedHeap` setup and makes it reusable
/// across VNodes with a consistent API.
pub struct VNodeHeap {
    inner: LockedHeap,
}

unsafe impl GlobalAlloc for VNodeHeap {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        self.inner.alloc(layout)
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        self.inner.dealloc(ptr, layout)
    }
}

impl VNodeHeap {
    pub const fn new() -> Self {
        Self {
            inner: LockedHeap::empty(),
        }
    }

    /// Initialize the heap from a raw heap region.
    ///
    /// # Safety
    ///
    /// - `heap_start` must point to a valid writable memory region of at least
    ///   `heap_size` bytes.
    /// - `heap_start` must be aligned to `usize`.
    pub unsafe fn init(&self, heap_start: *mut u8, heap_size: usize) {
        let align = core::mem::align_of::<usize>();
        debug_assert!(heap_start as usize % align == 0, "Heap start must be aligned");
        debug_assert!(heap_size >= 4096, "Heap size should be at least 4 KiB");
        self.inner.lock().init(heap_start, heap_size);
    }

    /// Initialize the heap from a static buffer reference.
    pub unsafe fn init_buffer(&self, buffer: &mut [u8]) {
        self.init(buffer.as_mut_ptr(), buffer.len());
    }
}
