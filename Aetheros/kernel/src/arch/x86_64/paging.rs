#![allow(dead_code)]

use conquer_once::spin::Once;
use spin::Mutex;
use x86_64::registers::control::Cr3;
use x86_64::structures::paging::{
    mapper::MapToError, FrameAllocator, Mapper, OffsetPageTable, Page, PageTable, PageTableFlags,
    PhysFrame, Size4KiB,
};
use x86_64::{PhysAddr, VirtAddr};

use crate::kprintln;
use crate::memory::frame_allocator::GlobalFrameAllocator;

const MAX_ADDRESS_SPACES: usize = 64;
const KERNEL_PML4_ENTRY_START: usize = 256;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AddressSpaceKind {
    Kernel,
    User,
}

#[derive(Clone, Copy, Debug)]
pub struct AddressSpace {
    pub asid: u16,
    pub kind: AddressSpaceKind,
    pub root_frame: PhysFrame,
}

#[derive(Debug)]
pub enum PagingError {
    MissingPhysicalMemoryOffset,
    AddressSpaceLimitReached,
    FrameAllocationFailed,
    MapFailed,
    UnmapFailed,
}

pub struct PageTableManager {
    physical_memory_offset: VirtAddr,
    next_asid: u16,
    spaces: [Option<AddressSpace>; MAX_ADDRESS_SPACES],
}

impl PageTableManager {
    pub fn new(physical_memory_offset: VirtAddr) -> Self {
        let active_root = Cr3::read().0;
        let kernel_space = AddressSpace {
            asid: 0,
            kind: AddressSpaceKind::Kernel,
            root_frame: active_root,
        };

        let mut spaces = [None; MAX_ADDRESS_SPACES];
        spaces[0] = Some(kernel_space);

        Self {
            physical_memory_offset,
            next_asid: 1,
            spaces,
        }
    }

    pub fn create_user_address_space(&mut self) -> Result<AddressSpace, PagingError> {
        let asid = self.allocate_asid()?;
        let mut frame_alloc = GlobalFrameAllocator;
        let new_root = frame_alloc
            .allocate_frame()
            .ok_or(PagingError::FrameAllocationFailed)?;

        unsafe {
            let new_table = self.table_ptr_mut(new_root);
            core::ptr::write_bytes(new_table as *mut u8, 0, core::mem::size_of::<PageTable>());

            let kernel_root = self.table_ptr(Cr3::read().0);
            for idx in KERNEL_PML4_ENTRY_START..512 {
                (*new_table)[idx] = (*kernel_root)[idx].clone();
            }
        }

        let space = AddressSpace {
            asid,
            kind: AddressSpaceKind::User,
            root_frame: new_root,
        };

        self.store_space(space)?;
        Ok(space)
    }

    pub fn map_in_kernel(
        &mut self,
        page: Page<Size4KiB>,
        frame: PhysFrame,
        mut flags: PageTableFlags,
    ) -> Result<(), PagingError> {
        flags |= PageTableFlags::GLOBAL;
        let mut frame_alloc = GlobalFrameAllocator;
        let flush = unsafe {
            self.kernel_mapper()
                .map_to(page, frame, flags, &mut frame_alloc)
                .map_err(map_to_err)?
        };
        flush.flush();
        Ok(())
    }

    pub fn map_in_user(
        &mut self,
        space: AddressSpace,
        page: Page<Size4KiB>,
        frame: PhysFrame,
        mut flags: PageTableFlags,
    ) -> Result<(), PagingError> {
        flags |= PageTableFlags::USER_ACCESSIBLE;
        let mut frame_alloc = GlobalFrameAllocator;
        let flush = unsafe {
            self.mapper_for_root(space.root_frame)
                .map_to(page, frame, flags, &mut frame_alloc)
                .map_err(map_to_err)?
        };
        flush.flush();
        Ok(())
    }

    pub fn unmap_kernel(&mut self, page: Page<Size4KiB>) -> Result<PhysFrame, PagingError> {
        let (frame, flush) = unsafe {
            self.kernel_mapper()
                .unmap(page)
                .map_err(|_| PagingError::UnmapFailed)?
        };
        flush.flush();
        Ok(frame)
    }

    unsafe fn kernel_mapper(&mut self) -> OffsetPageTable<'static> {
        OffsetPageTable::new(
            self.table_ptr_mut(Cr3::read().0),
            self.physical_memory_offset,
        )
    }

    unsafe fn mapper_for_root(&mut self, root: PhysFrame) -> OffsetPageTable<'static> {
        OffsetPageTable::new(self.table_ptr_mut(root), self.physical_memory_offset)
    }

    unsafe fn table_ptr(&self, frame: PhysFrame) -> *const PageTable {
        let virt = self.physical_memory_offset + frame.start_address().as_u64();
        virt.as_ptr::<PageTable>()
    }

    unsafe fn table_ptr_mut(&self, frame: PhysFrame) -> &'static mut PageTable {
        let virt = self.physical_memory_offset + frame.start_address().as_u64();
        &mut *virt.as_mut_ptr::<PageTable>()
    }

    fn allocate_asid(&mut self) -> Result<u16, PagingError> {
        let asid = self.next_asid;
        if asid as usize >= MAX_ADDRESS_SPACES {
            return Err(PagingError::AddressSpaceLimitReached);
        }
        self.next_asid = self.next_asid.saturating_add(1);
        Ok(asid)
    }

    fn store_space(&mut self, space: AddressSpace) -> Result<(), PagingError> {
        let slot = self.spaces.iter_mut().find(|entry| entry.is_none());
        if let Some(entry) = slot {
            *entry = Some(space);
            Ok(())
        } else {
            Err(PagingError::AddressSpaceLimitReached)
        }
    }
}

fn map_to_err(error: MapToError<Size4KiB>) -> PagingError {
    match error {
        MapToError::FrameAllocationFailed => PagingError::FrameAllocationFailed,
        _ => PagingError::MapFailed,
    }
}

static PAGE_TABLE_MANAGER: Once<Mutex<PageTableManager>> = Once::new();

pub fn init(physical_memory_offset: Option<u64>) -> Result<(), PagingError> {
    let offset = physical_memory_offset.ok_or(PagingError::MissingPhysicalMemoryOffset)?;
    PAGE_TABLE_MANAGER.call_once(|| Mutex::new(PageTableManager::new(VirtAddr::new(offset))));
    kprintln!(
        "[kernel] paging: initialized with physical_memory_offset={:#x}",
        offset
    );
    Ok(())
}

pub fn with_manager<R>(f: impl FnOnce(&mut PageTableManager) -> R) -> R {
    let manager = PAGE_TABLE_MANAGER
        .get()
        .expect("Paging manager not initialized");
    let mut guard = manager.lock();
    f(&mut guard)
}

pub fn create_user_address_space() -> Result<AddressSpace, PagingError> {
    with_manager(|mgr| mgr.create_user_address_space())
}

pub fn map_kernel_page(
    virtual_address: VirtAddr,
    physical_address: PhysAddr,
    flags: PageTableFlags,
) -> Result<(), PagingError> {
    with_manager(|mgr| {
        mgr.map_in_kernel(
            Page::containing_address(virtual_address),
            PhysFrame::containing_address(physical_address),
            flags,
        )
    })
}

pub fn map_user_page(
    space: AddressSpace,
    virtual_address: VirtAddr,
    physical_address: PhysAddr,
    flags: PageTableFlags,
) -> Result<(), PagingError> {
    with_manager(|mgr| {
        mgr.map_in_user(
            space,
            Page::containing_address(virtual_address),
            PhysFrame::containing_address(physical_address),
            flags,
        )
    })
}

pub fn unmap_kernel_page(virtual_address: VirtAddr) -> Result<PhysFrame, PagingError> {
    with_manager(|mgr| mgr.unmap_kernel(Page::containing_address(virtual_address)))
}
