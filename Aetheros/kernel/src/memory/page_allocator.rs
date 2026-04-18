#![allow(dead_code)]

use x86_64::structures::paging::{FrameAllocator, FrameDeallocator, PageTableFlags};
use x86_64::{PhysAddr, VirtAddr};

use crate::arch::x86_64::paging::{self, AddressSpace};
use crate::kprintln;
use crate::memory::frame_allocator::GlobalFrameAllocator;

/// Page allocator that maps pages into kernel or user address spaces.
pub struct PageAllocator;

impl PageAllocator {
    pub const fn new() -> Self {
        Self
    }

    pub fn init() {
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
}
