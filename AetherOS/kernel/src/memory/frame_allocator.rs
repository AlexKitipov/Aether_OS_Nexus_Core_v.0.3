#![allow(dead_code)]

extern crate alloc;

use bootloader_api::info::{MemoryRegionKind, MemoryRegions};
use conquer_once::spin::Once;
use spin::Mutex;
use x86_64::structures::paging::{FrameAllocator, FrameDeallocator, PhysFrame, Size4KiB};
use x86_64::PhysAddr;

use crate::kprintln;

const FRAME_SIZE: u64 = Size4KiB::SIZE;
const MAX_TRACKED_FRAMES: usize = 1 << 20; // 4GiB / 4KiB
const BITMAP_WORDS: usize = MAX_TRACKED_FRAMES / 64;

#[derive(Debug)]
pub enum FrameAllocatorError {
    NoUsableMemory,
    AddressSpaceTooLarge,
    FrameOutOfRange,
    DoubleFree,
}

/// Bitmap-backed frame allocator populated from bootloader memory regions.
/// A set bit means allocated, clear means free.
pub struct BootInfoFrameAllocator {
    base_phys: u64,
    frame_count: usize,
    next_hint: usize,
    free_frames: usize,
    bitmap: [u64; BITMAP_WORDS],
}

impl BootInfoFrameAllocator {
    pub const fn empty() -> Self {
        Self {
            base_phys: 0,
            frame_count: 0,
            next_hint: 0,
            free_frames: 0,
            bitmap: [u64::MAX; BITMAP_WORDS],
        }
    }

    /// # Safety
    /// `memory_regions` must represent the active platform memory map.
    pub unsafe fn init(memory_regions: &'static MemoryRegions) -> Result<Self, FrameAllocatorError> {
        let mut min_usable = u64::MAX;
        let mut max_usable = 0u64;

        for region in memory_regions
            .iter()
            .filter(|r| r.kind == MemoryRegionKind::Usable && r.end > r.start)
        {
            min_usable = min_usable.min(region.start);
            max_usable = max_usable.max(region.end);
        }

        if min_usable == u64::MAX || max_usable <= min_usable {
            return Err(FrameAllocatorError::NoUsableMemory);
        }

        let aligned_start = align_up(min_usable, FRAME_SIZE);
        let aligned_end = align_down(max_usable, FRAME_SIZE);
        if aligned_end <= aligned_start {
            return Err(FrameAllocatorError::NoUsableMemory);
        }

        let frame_span = ((aligned_end - aligned_start) / FRAME_SIZE) as usize;
        if frame_span > MAX_TRACKED_FRAMES {
            return Err(FrameAllocatorError::AddressSpaceTooLarge);
        }

        let mut allocator = Self::empty();
        allocator.base_phys = aligned_start;
        allocator.frame_count = frame_span;
        allocator.next_hint = 0;

        for idx in 0..allocator.frame_count {
            allocator.set_allocated(idx);
        }

        for region in memory_regions
            .iter()
            .filter(|r| r.kind == MemoryRegionKind::Usable && r.end > r.start)
        {
            let start = align_up(region.start, FRAME_SIZE).max(aligned_start);
            let end = align_down(region.end, FRAME_SIZE).min(aligned_end);
            if end <= start {
                continue;
            }

            let mut addr = start;
            while addr < end {
                let idx = ((addr - allocator.base_phys) / FRAME_SIZE) as usize;
                allocator.set_free(idx);
                addr += FRAME_SIZE;
            }
        }

        allocator.free_frames = allocator.count_free_frames();
        kprintln!(
            "[kernel] frame_allocator: Ready. base={:#x}, frames={}, free={}",
            allocator.base_phys,
            allocator.frame_count,
            allocator.free_frames
        );

        Ok(allocator)
    }

    pub fn total_frames(&self) -> usize {
        self.frame_count
    }

    pub fn free_frames(&self) -> usize {
        self.free_frames
    }

    pub fn allocate(&mut self) -> Option<PhysFrame<Size4KiB>> {
        if self.free_frames == 0 {
            return None;
        }

        let start = self.next_hint;
        for step in 0..self.frame_count {
            let idx = (start + step) % self.frame_count;
            if !self.is_allocated(idx) {
                self.set_allocated(idx);
                self.free_frames -= 1;
                self.next_hint = (idx + 1) % self.frame_count;
                return Some(self.index_to_frame(idx));
            }
        }

        None
    }

    pub fn deallocate(&mut self, frame: PhysFrame<Size4KiB>) -> Result<(), FrameAllocatorError> {
        let idx = self.frame_to_index(frame)?;
        if !self.is_allocated(idx) {
            return Err(FrameAllocatorError::DoubleFree);
        }

        self.set_free(idx);
        self.free_frames += 1;
        if idx < self.next_hint {
            self.next_hint = idx;
        }
        Ok(())
    }

    fn frame_to_index(&self, frame: PhysFrame<Size4KiB>) -> Result<usize, FrameAllocatorError> {
        let addr = frame.start_address().as_u64();
        if addr < self.base_phys {
            return Err(FrameAllocatorError::FrameOutOfRange);
        }
        let delta = addr - self.base_phys;
        if delta % FRAME_SIZE != 0 {
            return Err(FrameAllocatorError::FrameOutOfRange);
        }
        let idx = (delta / FRAME_SIZE) as usize;
        if idx >= self.frame_count {
            return Err(FrameAllocatorError::FrameOutOfRange);
        }
        Ok(idx)
    }

    fn index_to_frame(&self, idx: usize) -> PhysFrame<Size4KiB> {
        let addr = self.base_phys + idx as u64 * FRAME_SIZE;
        PhysFrame::containing_address(PhysAddr::new(addr))
    }

    fn is_allocated(&self, idx: usize) -> bool {
        let word = idx / 64;
        let bit = idx % 64;
        (self.bitmap[word] & (1u64 << bit)) != 0
    }

    fn set_allocated(&mut self, idx: usize) {
        let word = idx / 64;
        let bit = idx % 64;
        self.bitmap[word] |= 1u64 << bit;
    }

    fn set_free(&mut self, idx: usize) {
        let word = idx / 64;
        let bit = idx % 64;
        self.bitmap[word] &= !(1u64 << bit);
    }

    fn count_free_frames(&self) -> usize {
        (0..self.frame_count)
            .filter(|idx| !self.is_allocated(*idx))
            .count()
    }
}

unsafe impl FrameAllocator<Size4KiB> for BootInfoFrameAllocator {
    fn allocate_frame(&mut self) -> Option<PhysFrame<Size4KiB>> {
        self.allocate()
    }
}

impl FrameDeallocator<Size4KiB> for BootInfoFrameAllocator {
    unsafe fn deallocate_frame(&mut self, frame: PhysFrame<Size4KiB>) {
        let _ = self.deallocate(frame);
    }
}

pub struct GlobalFrameAllocator;

static FRAME_ALLOCATOR: Once<Mutex<BootInfoFrameAllocator>> = Once::new();

pub fn init_global(memory_regions: &'static MemoryRegions) -> Result<(), FrameAllocatorError> {
    let allocator = unsafe { BootInfoFrameAllocator::init(memory_regions)? };
    FRAME_ALLOCATOR.call_once(|| Mutex::new(allocator));
    Ok(())
}

pub fn with_allocator<R>(f: impl FnOnce(&mut BootInfoFrameAllocator) -> R) -> R {
    let alloc = FRAME_ALLOCATOR
        .get()
        .expect("frame allocator not initialized");
    let mut guard = alloc.lock();
    f(&mut guard)
}

unsafe impl FrameAllocator<Size4KiB> for GlobalFrameAllocator {
    fn allocate_frame(&mut self) -> Option<PhysFrame<Size4KiB>> {
        with_allocator(|alloc| alloc.allocate())
    }
}

impl FrameDeallocator<Size4KiB> for GlobalFrameAllocator {
    unsafe fn deallocate_frame(&mut self, frame: PhysFrame<Size4KiB>) {
        with_allocator(|alloc| {
            let _ = alloc.deallocate(frame);
        });
    }
}

#[inline]
const fn align_up(value: u64, align: u64) -> u64 {
    (value + (align - 1)) & !(align - 1)
}

#[inline]
const fn align_down(value: u64, align: u64) -> u64 {
    value & !(align - 1)
}
