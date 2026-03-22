#![allow(dead_code)] // Allow dead code for now as not all functions might be used immediately

extern crate alloc;

use crate::arch::x86_64::paging;
use crate::kprintln;
use crate::memory;
use crate::memory::frame_allocator::BootInfoFrameAllocator;
use core::sync::atomic::{AtomicU64, Ordering};
use spin::Mutex;
use x86_64::VirtAddr;

const PAGE_SIZE: u64 = 4096;
const PAGE_FLAGS_PRESENT_WRITABLE: u64 = 0x3;
const DYNAMIC_VIRT_START: u64 = 0xFFFF_9000_0000_0000;

/// Next available high-half virtual page for bootstrap allocations.
static NEXT_VIRT_PAGE: AtomicU64 = AtomicU64::new(DYNAMIC_VIRT_START);

/// Tracks virtual->physical bindings created through `PageAllocator`.
static PAGE_BINDINGS: Mutex<alloc::collections::BTreeMap<u64, u64>> =
    Mutex::new(alloc::collections::BTreeMap::new());

/// A page allocator that binds virtual pages to physical frames.
///
/// The allocator is bootstrap-oriented: it uses the global `BootInfoFrameAllocator`
/// and the architecture paging shim to install mappings.
pub struct PageAllocator {
    _private: (),
}

impl Default for PageAllocator {
    fn default() -> Self {
        Self::new()
    }
}

impl PageAllocator {
    /// Creates a new, uninitialized PageAllocator.
    pub const fn new() -> Self {
        PageAllocator { _private: () }
    }

    /// Initializes the Page Allocator.
    pub fn init(_frame_allocator: &mut BootInfoFrameAllocator) {
        kprintln!("[kernel] page_allocator: Initializing...");
        NEXT_VIRT_PAGE.store(DYNAMIC_VIRT_START, Ordering::SeqCst);
        PAGE_BINDINGS.lock().clear();
        kprintln!(
            "[kernel] page_allocator: Initialized at virtual base {:#x}.",
            DYNAMIC_VIRT_START
        );
    }

    /// Allocates one virtual page and maps it to a fresh physical frame.
    pub fn allocate_page() -> Option<VirtAddr> {
        let phys = memory::alloc_frame_addr()?.as_u64();
        let virt = NEXT_VIRT_PAGE.fetch_add(PAGE_SIZE, Ordering::SeqCst);

        // Install the mapping in the architecture paging layer.
        paging::map_page_real(phys, virt, PAGE_FLAGS_PRESENT_WRITABLE);

        PAGE_BINDINGS.lock().insert(virt, phys);

        kprintln!(
            "[kernel] page_allocator: mapped virt {:#x} -> phys {:#x}.",
            virt,
            phys
        );

        Some(VirtAddr::new(virt))
    }

    /// Deallocates a previously allocated virtual page.
    ///
    /// Bootstrap limitation: physical frames are not yet returned to the free pool,
    /// but virtual mappings are removed from the software translation table.
    pub fn deallocate_page(page_addr: VirtAddr) {
        let virt = page_addr.as_u64() & !(PAGE_SIZE - 1);
        let removed = PAGE_BINDINGS.lock().remove(&virt);

        if removed.is_some() {
            paging::unregister_virt_mapping(virt, PAGE_SIZE as usize);
            kprintln!("[kernel] page_allocator: unmapped virt {:#x}.", virt);
        } else {
            kprintln!(
                "[kernel] page_allocator: deallocate ignored, virt {:#x} not tracked.",
                virt
            );
        }
    }
}
