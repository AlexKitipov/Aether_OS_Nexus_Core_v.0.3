#![allow(dead_code)] // Allow dead code for now as not all functions might be used immediately

extern crate alloc;

use crate::kprintln;
use bootloader_api::info::{MemoryRegionKind, MemoryRegions};
use core::ops::Range;
use x86_64::structures::paging::{FrameAllocator, PageSize, PhysFrame, Size4KiB};
use x86_64::PhysAddr;

const FRAME_SIZE: usize = Size4KiB::SIZE as usize;

/// A FrameAllocator that returns usable frames from the bootloader's memory map.
///
/// This allocator iterates through the memory regions provided by the bootloader
/// and yields usable physical frames.
pub struct BootInfoFrameAllocator {
    usable_ranges: alloc::vec::Vec<Range<u64>>,
    next: usize,
}

impl BootInfoFrameAllocator {
    /// Create a FrameAllocator from the bootloader's memory map.
    ///
    /// This function is unsafe because the caller must guarantee that the passed
    /// memory regions are valid and represent the actual physical memory layout.
    pub unsafe fn init(memory_regions: &'static MemoryRegions) -> Self {
        kprintln!("[kernel] frame_allocator: Initializing BootInfoFrameAllocator...");
        let usable_ranges = collect_usable_ranges(memory_regions);
        assert!(
            !usable_ranges.is_empty(),
            "[kernel] frame_allocator: no usable RAM regions were provided by bootloader"
        );
        BootInfoFrameAllocator {
            usable_ranges,
            next: 0,
        }
    }

    /// Returns an iterator over the usable frames in the memory map.
    fn usable_frames(&self) -> impl Iterator<Item = PhysFrame> + '_ {
        // We only walk validated, page-aligned ranges so we never allocate frames
        // outside bootloader-advertised usable RAM.
        let frame_addresses = self
            .usable_ranges
            .iter()
            .flat_map(|r| (r.start..r.end).step_by(FRAME_SIZE).map(PhysAddr::new));

        // Create PhysFrame for each address
        frame_addresses.map(|addr| PhysFrame::containing_address(addr))
    }
}

fn align_up(value: u64, align: u64) -> u64 {
    ((value + (align - 1)) / align) * align
}

fn align_down(value: u64, align: u64) -> u64 {
    (value / align) * align
}

fn collect_usable_ranges(memory_regions: &'static MemoryRegions) -> alloc::vec::Vec<Range<u64>> {
    let mut usable = alloc::vec::Vec::new();
    let frame_size = FRAME_SIZE as u64;

    for region in memory_regions.iter() {
        if region.end <= region.start {
            kprintln!(
                "[kernel] frame_allocator: skipping invalid region {:#x?}..{:#x?} ({:?}).",
                region.start,
                region.end,
                region.kind
            );
            continue;
        }

        if region.kind != MemoryRegionKind::Usable {
            continue;
        }

        let start = align_up(region.start, frame_size);
        let end = align_down(region.end, frame_size);
        if end <= start {
            kprintln!(
                "[kernel] frame_allocator: skipping tiny/unaligned usable region {:#x?}..{:#x?}.",
                region.start,
                region.end
            );
            continue;
        }

        usable.push(start..end);
    }

    // Bootloader maps are typically ordered, but we must not assume this.
    // We normalize and merge to avoid double-allocating overlapping frames.
    usable.sort_unstable_by_key(|r| r.start);
    let mut normalized: alloc::vec::Vec<Range<u64>> = alloc::vec::Vec::new();

    for range in usable {
        if let Some(last) = normalized.last_mut() {
            if range.start <= last.end {
                if range.end > last.end {
                    kprintln!(
                        "[kernel] frame_allocator: merged overlapping usable ranges {:#x}..{:#x} and {:#x}..{:#x}.",
                        last.start,
                        last.end,
                        range.start,
                        range.end
                    );
                    last.end = range.end;
                }
                continue;
            }
        }
        normalized.push(range);
    }

    normalized
}

// Implement the `FrameAllocator` trait for `BootInfoFrameAllocator`.
// This is crucial for integrating with `x86_64` paging structures.
unsafe impl FrameAllocator<Size4KiB> for BootInfoFrameAllocator {
    fn allocate_frame(&mut self) -> Option<PhysFrame> {
        // Iterate through usable frames and return the next available one.
        let frame = self.usable_frames().nth(self.next);
        frame.inspect(|_| {
            self.next += 1;
        })
    }
}
