#![allow(dead_code)] // Allow dead code for now as not all functions might be used immediately

use crate::kprintln;
use bootloader_api::info::{MemoryRegionKind, MemoryRegions};
use x86_64::structures::paging::{FrameAllocator, PhysFrame, Size4KiB};
use x86_64::PhysAddr;

/// A FrameAllocator that returns usable frames from the bootloader's memory map.
///
/// This allocator iterates through the memory regions provided by the bootloader
/// and yields usable physical frames.
pub struct BootInfoFrameAllocator {
    memory_regions: &'static MemoryRegions,
    next: usize,
}

impl BootInfoFrameAllocator {
    /// Create a FrameAllocator from the bootloader's memory map.
    ///
    /// This function is unsafe because the caller must guarantee that the passed
    /// memory regions are valid and represent the actual physical memory layout.
    pub unsafe fn init(memory_regions: &'static MemoryRegions) -> Self {
        kprintln!("[kernel] frame_allocator: Initializing BootInfoFrameAllocator...");
        BootInfoFrameAllocator {
            memory_regions,
            next: 0,
        }
    }

    /// Returns an iterator over the usable frames in the memory map.
    fn usable_frames(&self) -> impl Iterator<Item = PhysFrame> {
        // Get usable regions from memory map
        let regions = self
            .memory_regions
            .iter()
            .filter(|r| r.kind == MemoryRegionKind::Usable && r.end > r.start);

        // Map each region to its address range
        let addr_ranges = regions.map(|r| r.start..r.end);

        // Transform to an iterator of frame start addresses
        let frame_addresses = addr_ranges.flat_map(|r| r.step_by(4096).map(PhysAddr::new));

        // Create PhysFrame for each address
        frame_addresses.map(|addr| PhysFrame::containing_address(addr))
    }
}

// Implement the `FrameAllocator` trait for `BootInfoFrameAllocator`.
// This is crucial for integrating with `x86_64` paging structures.
unsafe impl FrameAllocator<Size4KiB> for BootInfoFrameAllocator {
    fn allocate_frame(&mut self) -> Option<PhysFrame> {
        // Iterate through usable frames and return the next available one.
        let frame = self.usable_frames().nth(self.next);
        if frame.is_some() {
            self.next += 1;
        }
        frame
    }
}

