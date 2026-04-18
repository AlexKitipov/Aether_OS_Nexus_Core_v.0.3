#![allow(dead_code)]

use x86_64::structures::paging::{FrameAllocator, FrameDeallocator, PageTableFlags};
use x86_64::{PhysAddr, VirtAddr};

use crate::arch::x86_64::paging::{self, AddressSpace};
use crate::kprintln;
use crate::memory::frame_allocator::GlobalFrameAllocator;

const PAGE_SIZE: u64 = 4096;
const DYNAMIC_VIRT_START: u64 = 0xFFFF_9000_0000_0000;

static NEXT_DYNAMIC_VIRT: core::sync::atomic::AtomicU64 =
    core::sync::atomic::AtomicU64::new(DYNAMIC_VIRT_START);

/// Page allocator that maps pages into kernel or user address spaces.
pub struct PageAllocator;

impl Default for PageAllocator {
    fn default() -> Self {
        Self::new()
    }
}

impl PageAllocator {
    pub const fn new() -> Self {
        Self
    }

    pub fn init() {
        NEXT_DYNAMIC_VIRT.store(DYNAMIC_VIRT_START, core::sync::atomic::Ordering::SeqCst);
        kprintln!("[kernel] page_allocator: Initialized.");
    }

    pub fn allocate_kernel_page(
        virtual_address: VirtAddr,
        flags: PageTableFlags,
    ) -> Option<PhysAddr> {
        let mut allocator = GlobalFrameAllocator;
        let frame = allocator.allocate_frame()?;
        let physical = frame.start_address();

        paging::map_kernel_page(
            virtual_address,
            physical,
            flags | PageTableFlags::PRESENT | PageTableFlags::WRITABLE,
        )
        .ok()?;

        Some(physical)
    }

    pub fn allocate_user_page(
        address_space: AddressSpace,
        virtual_address: VirtAddr,
        flags: PageTableFlags,
    ) -> Option<PhysAddr> {
        let mut allocator = GlobalFrameAllocator;
        let frame = allocator.allocate_frame()?;
        let physical = frame.start_address();

        paging::map_user_page(
            address_space,
            virtual_address,
            physical,
            flags | PageTableFlags::PRESENT | PageTableFlags::WRITABLE,
        )
        .ok()?;

        Some(physical)
    }

    pub fn deallocate_kernel_page(virtual_address: VirtAddr) {
        if let Ok(frame) = paging::unmap_kernel_page(virtual_address) {
            unsafe {
                let mut allocator = GlobalFrameAllocator;
                allocator.deallocate_frame(frame);
            }
        }
    }

    /// Compatibility helper retained for current scheduler/vnode call sites.
    pub fn allocate_page() -> Option<VirtAddr> {
        let virt = NEXT_DYNAMIC_VIRT.fetch_add(PAGE_SIZE, core::sync::atomic::Ordering::SeqCst);
        Self::allocate_kernel_page(VirtAddr::new(virt), PageTableFlags::empty())?;
        Some(VirtAddr::new(virt))
    }

    /// Compatibility helper retained for current scheduler/vnode call sites.
    pub fn deallocate_page(page_addr: VirtAddr) {
        Self::deallocate_kernel_page(VirtAddr::new(page_addr.as_u64() & !(PAGE_SIZE - 1)));
    }
}
