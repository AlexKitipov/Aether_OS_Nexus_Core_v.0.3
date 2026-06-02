#![allow(dead_code)]

extern crate alloc;

use alloc::string::String;
use alloc::vec::Vec;
use x86_64::structures::paging::{FrameAllocator, PageTableFlags, PhysFrame, Size4KiB};
use x86_64::{PhysAddr, VirtAddr};

use crate::arch::x86_64::paging::{self, AddressSpace, PagingError};
use crate::memory::frame_allocator::GlobalFrameAllocator;

pub const USER_CODE_BASE: u64 = 0x0000_0000_4000_0000;
pub const USER_STACK_TOP: u64 = 0x0000_7fff_ffff_f000;
pub const DEFAULT_USER_STACK_PAGES: usize = 1;
const PAGE_SIZE: u64 = 4096;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct UserSegmentFlags {
    pub writable: bool,
    pub executable: bool,
}

impl UserSegmentFlags {
    pub const READ_ONLY: Self = Self {
        writable: false,
        executable: false,
    };

    pub const READ_WRITE: Self = Self {
        writable: true,
        executable: false,
    };

    pub const EXECUTABLE: Self = Self {
        writable: false,
        executable: true,
    };

    fn page_table_flags(self) -> PageTableFlags {
        let mut flags = PageTableFlags::PRESENT;
        if self.writable {
            flags |= PageTableFlags::WRITABLE;
        }
        if !self.executable {
            flags |= PageTableFlags::NO_EXECUTE;
        }
        flags
    }
}

#[derive(Clone, Copy, Debug)]
pub struct UserSegment<'a> {
    pub virtual_start: VirtAddr,
    pub bytes: &'a [u8],
    pub flags: UserSegmentFlags,
}

#[derive(Debug)]
pub enum AddressSpaceError {
    Paging(PagingError),
    FrameAllocationFailed,
    MissingPhysicalMemoryOffset,
    InvalidSegment,
}

impl From<PagingError> for AddressSpaceError {
    fn from(error: PagingError) -> Self {
        Self::Paging(error)
    }
}

impl From<AddressSpaceError> for String {
    fn from(error: AddressSpaceError) -> Self {
        match error {
            AddressSpaceError::Paging(err) => alloc::format!("paging error: {:?}", err),
            AddressSpaceError::FrameAllocationFailed => String::from("failed to allocate physical frame"),
            AddressSpaceError::MissingPhysicalMemoryOffset => {
                String::from("paging physical-memory offset is not configured")
            }
            AddressSpaceError::InvalidSegment => String::from("invalid unaligned or overflowing user segment"),
        }
    }
}

#[derive(Debug, Clone)]
pub struct ManagedAddressSpace {
    pub space: AddressSpace,
    pub root_frame: PhysAddr,
    pub user_stack_base: VirtAddr,
    pub user_stack_top: VirtAddr,
    pub mapped_pages: Vec<VirtAddr>,
    pub owned_frames: Vec<PhysAddr>,
}

impl ManagedAddressSpace {
    pub fn root_pml4(&self) -> u64 {
        self.root_frame.as_u64()
    }

    fn track_mapping(&mut self, page: VirtAddr, frame: PhysAddr) {
        self.mapped_pages.push(page);
        self.owned_frames.push(frame);
    }
}

/// Builds a minimal isolated user address space for a V-Node.
///
/// The architecture paging layer creates a fresh PML4 and copies only the
/// kernel half of the active root table into it. User pages are then mapped with
/// `USER_ACCESSIBLE`; kernel mappings are never given that flag.
pub fn create_vnode_address_space(
    segments: &[UserSegment<'_>],
    stack_pages: usize,
) -> Result<ManagedAddressSpace, AddressSpaceError> {
    let space = paging::create_user_address_space()?;
    let root_frame = space.root_frame.start_address();
    let stack_pages = stack_pages.max(1);
    let stack_top = VirtAddr::new(USER_STACK_TOP);
    let stack_size = (stack_pages as u64)
        .checked_mul(PAGE_SIZE)
        .ok_or(AddressSpaceError::InvalidSegment)?;
    let stack_base = VirtAddr::new(
        stack_top
            .as_u64()
            .checked_sub(stack_size)
            .ok_or(AddressSpaceError::InvalidSegment)?,
    );

    let mut managed = ManagedAddressSpace {
        space,
        root_frame,
        user_stack_base: stack_base,
        user_stack_top: stack_top,
        mapped_pages: Vec::new(),
        owned_frames: Vec::new(),
    };
    managed.owned_frames.push(root_frame);

    for segment in segments {
        map_user_segment(&mut managed, *segment)?;
    }

    map_user_stack(&mut managed, stack_base, stack_pages)?;
    Ok(managed)
}

pub fn map_user_stack(
    managed: &mut ManagedAddressSpace,
    stack_base: VirtAddr,
    page_count: usize,
) -> Result<(), AddressSpaceError> {
    for page_idx in 0..page_count.max(1) {
        let page = stack_base + (page_idx as u64 * PAGE_SIZE);
        let frame = allocate_zeroed_frame()?;
        let page_table_frames = paging::map_user_page_tracked(
            managed.space,
            page,
            frame.start_address(),
            PageTableFlags::PRESENT | PageTableFlags::WRITABLE | PageTableFlags::NO_EXECUTE,
        )?;
        managed.track_mapping(page, frame.start_address());
        managed.owned_frames.extend(page_table_frames);
    }
    Ok(())
}

pub fn map_user_segment(
    managed: &mut ManagedAddressSpace,
    segment: UserSegment<'_>,
) -> Result<(), AddressSpaceError> {
    if segment.bytes.is_empty() {
        return Ok(());
    }

    let start = segment.virtual_start.as_u64();
    let end = start
        .checked_add(segment.bytes.len() as u64)
        .ok_or(AddressSpaceError::InvalidSegment)?;
    let first_page = align_down(start, PAGE_SIZE);
    let last_page = align_down(end.saturating_sub(1), PAGE_SIZE);
    let mut copied = 0usize;
    let flags = segment.flags.page_table_flags();

    let mut page = first_page;
    while page <= last_page {
        let frame = allocate_zeroed_frame()?;
        let page_offset = if page == first_page {
            (start - first_page) as usize
        } else {
            0
        };
        let copy_len = core::cmp::min(
            PAGE_SIZE as usize - page_offset,
            segment.bytes.len().saturating_sub(copied),
        );
        copy_to_frame(frame, page_offset, &segment.bytes[copied..copied + copy_len])?;

        let page_table_frames = paging::map_user_page_tracked(
            managed.space,
            VirtAddr::new(page),
            frame.start_address(),
            flags,
        )?;
        managed.track_mapping(VirtAddr::new(page), frame.start_address());
        managed.owned_frames.extend(page_table_frames);

        copied += copy_len;
        page = page
            .checked_add(PAGE_SIZE)
            .ok_or(AddressSpaceError::InvalidSegment)?;
    }

    Ok(())
}

fn allocate_zeroed_frame() -> Result<PhysFrame<Size4KiB>, AddressSpaceError> {
    let mut allocator = GlobalFrameAllocator;
    let frame = allocator
        .allocate_frame()
        .ok_or(AddressSpaceError::FrameAllocationFailed)?;
    zero_frame(frame)?;
    Ok(frame)
}

fn zero_frame(frame: PhysFrame<Size4KiB>) -> Result<(), AddressSpaceError> {
    let offset = paging::physical_memory_offset().ok_or(AddressSpaceError::MissingPhysicalMemoryOffset)?;
    let virt = VirtAddr::new(offset + frame.start_address().as_u64());
    unsafe {
        core::ptr::write_bytes(virt.as_mut_ptr::<u8>(), 0, PAGE_SIZE as usize);
    }
    Ok(())
}

fn copy_to_frame(
    frame: PhysFrame<Size4KiB>,
    offset_in_frame: usize,
    bytes: &[u8],
) -> Result<(), AddressSpaceError> {
    if offset_in_frame + bytes.len() > PAGE_SIZE as usize {
        return Err(AddressSpaceError::InvalidSegment);
    }
    let offset = paging::physical_memory_offset().ok_or(AddressSpaceError::MissingPhysicalMemoryOffset)?;
    let virt = VirtAddr::new(offset + frame.start_address().as_u64() + offset_in_frame as u64);
    unsafe {
        core::ptr::copy_nonoverlapping(bytes.as_ptr(), virt.as_mut_ptr::<u8>(), bytes.len());
    }
    Ok(())
}

#[inline]
const fn align_down(value: u64, align: u64) -> u64 {
    value & !(align - 1)
}
