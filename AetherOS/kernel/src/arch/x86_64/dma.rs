// kernel/src/arch/x86_64/dma.rs

#![allow(dead_code)] // Allow dead code for now as not all functions might be used immediately

extern crate alloc;
use alloc::collections::BTreeMap;
use alloc::vec::Vec;
use core::sync::atomic::{AtomicU64, Ordering};
use spin::Mutex;
use crate::kprintln;
use crate::arch::x86_64::paging;

/// A simple DMA buffer manager for simulation.
/// In a real system, this would manage physically contiguous memory pages
/// and provide their physical addresses to devices.
/// For V-Nodes, these buffers are mapped into their virtual address space.

/// Static counter for generating unique DMA buffer handles.
static NEXT_HANDLE: AtomicU64 = AtomicU64::new(1);

#[derive(Debug)]
struct DmaBuffer {
    bytes: Vec<u8>,
    phys_base: u64,
}

/// Stores the allocated DMA buffers, mapped by their unique handles.
/// The `Vec<u8>` acts as the memory backing for the DMA buffer.
static DMA_BUFFERS: Mutex<BTreeMap<u64, DmaBuffer>> = Mutex::new(BTreeMap::new());

/// Allocates a new DMA-capable buffer of the specified `size`.
/// Returns a unique handle to the buffer, or `None` if allocation fails.
///
/// In a real system, this would involve allocating physically contiguous memory.
pub fn alloc_dma_buffer(size: usize) -> Option<u64> {
    let handle = NEXT_HANDLE.fetch_add(1, Ordering::SeqCst);
    let mut buffers = DMA_BUFFERS.lock();

    // Allocate and explicitly zero the backing storage.
    // This avoids exposing stale memory contents to DMA consumers.
    let mut buffer = Vec::with_capacity(size);
    buffer.resize(size, 0);
    let virt_addr = buffer.as_ptr() as u64;
    let phys_base = paging::alloc_frame_range(size);
    paging::register_virt_mapping(virt_addr, phys_base, size);

    buffers.insert(
        handle,
        DmaBuffer {
            bytes: buffer,
            phys_base,
        },
    );

    kprintln!(
        "[kernel] dma: Allocated & zeroed buffer handle {} ({} bytes).",
        handle,
        size
    );
    Some(handle)
}

/// Frees the DMA buffer associated with the given `handle`.
pub fn free_dma_buffer(handle: u64) {
    let mut buffers = DMA_BUFFERS.lock();
    if let Some(buf) = buffers.remove(&handle) {
        let virt_addr = buf.bytes.as_ptr() as u64;
        let size = buf.bytes.capacity();
        paging::unregister_virt_mapping(virt_addr, size);
        kprintln!("[kernel] dma: Freed buffer with handle {}.", handle);
    } else {
        kprintln!("[kernel] dma: Attempted to free non-existent buffer with handle {}.", handle);
    }
}

/// Returns a mutable raw pointer to the start of the DMA buffer.
/// This pointer would typically be a virtual address for the V-Node,
/// but for the kernel, it's the direct address of the `Vec`'s data.
pub fn get_dma_buffer_ptr(handle: u64) -> Option<*mut u8> {
    let mut buffers = DMA_BUFFERS.lock();
    buffers.get_mut(&handle).map(|buf| buf.bytes.as_mut_ptr())
}

/// Returns the physical address of a DMA buffer using an explicit direct-map
/// offset supplied by the caller.
pub fn get_phys_addr(handle: u64, physical_memory_offset: u64) -> Option<u64> {
    let buffers = DMA_BUFFERS.lock();
    buffers.get(&handle).map(|buf| {
        let virt_addr = buf.bytes.as_ptr() as u64;
        let translated = virt_addr.saturating_sub(physical_memory_offset);

        if translated != buf.phys_base {
            kprintln!(
                "[kernel] dma: translation mismatch for handle {} (virt {:#x} -> phys {:#x}, tracked base {:#x}).",
                handle,
                virt_addr,
                translated,
                buf.phys_base
            );
        }

        translated
    })
}

/// Backward-compatible helper that attempts to use explicit bootstrap mappings.
pub fn get_phys_addr_bootstrap(handle: u64) -> Option<u64> {
    let buffers = DMA_BUFFERS.lock();
    buffers.get(&handle).and_then(|buf| {
        let virt_addr = buf.bytes.as_ptr() as u64;
        paging::try_virt_to_phys(virt_addr)
    })
}


/// Clears a DMA buffer by zeroing all bytes.
///
/// This is useful when recycling buffers to avoid leaking stale data.
pub fn clear_buffer(handle: u64) {
    let mut buffers = DMA_BUFFERS.lock();
    if let Some(buf) = buffers.get_mut(&handle) {
        buf.bytes.fill(0);
        kprintln!("[kernel] dma: Cleared buffer with handle {}.", handle);
    } else {
        kprintln!("[kernel] dma: Attempted to clear non-existent buffer with handle {}.", handle);
    }
}

/// Returns the current capacity (allocated size) of the DMA buffer.
pub fn get_dma_buffer_capacity(handle: u64) -> Option<usize> {
    let buffers = DMA_BUFFERS.lock();
    buffers.get(&handle).map(|buf| buf.bytes.capacity())
}

/// Sets the effective length of the data within the DMA buffer.
/// This is used to indicate how much of the buffer is currently valid data.
pub fn set_dma_buffer_len(handle: u64, len: usize) -> Result<(), &'static str> {
    let mut buffers = DMA_BUFFERS.lock();
    if let Some(buf) = buffers.get_mut(&handle) {
        if len <= buf.bytes.capacity() {
            // SAFETY: We checked `len <= capacity`, so this is safe.
            // This is crucial for `Vec` to function correctly as a buffer.
            unsafe { buf.bytes.set_len(len); }
            kprintln!("[kernel] dma: Set length for handle {} to {}.", handle, len);
            Ok(())
        } else {
            kprintln!("[kernel] dma: Error setting length for handle {}: {} exceeds capacity {}.", handle, len, buf.bytes.capacity());
            Err("Length exceeds capacity")
        }
    } else {
        kprintln!("[kernel] dma: Error setting length: DMA handle {} not found.", handle);
        Err("DMA handle not found")
    }
}

/// Returns the current length (used size) of the DMA buffer.
pub fn get_dma_buffer_len(handle: u64) -> Option<usize> {
    let buffers = DMA_BUFFERS.lock();
    buffers.get(&handle).map(|buf| buf.bytes.len())
}

/// Copies data from a source slice into the DMA buffer.
pub fn write_to_buffer(handle: u64, data: &[u8], offset: usize) -> Result<(), &'static str> {
    let mut buffers = DMA_BUFFERS.lock();
    if let Some(buf) = buffers.get_mut(&handle) {
        if offset + data.len() <= buf.bytes.len() {
            buf.bytes[offset..offset + data.len()].copy_from_slice(data);
            Ok(())
        } else {
            Err("Out of bounds write")
        }
    } else {
        Err("Invalid handle")
    }
}

/// Copies data from the DMA buffer into a destination slice.
pub fn read_from_buffer(
    handle: u64,
    out_data: &mut [u8],
    offset: usize,
) -> Result<(), &'static str> {
    let buffers = DMA_BUFFERS.lock();
    if let Some(buf) = buffers.get(&handle) {
        if offset + out_data.len() <= buf.bytes.len() {
            out_data.copy_from_slice(&buf.bytes[offset..offset + out_data.len()]);
            Ok(())
        } else {
            Err("Out of bounds read")
        }
    } else {
        Err("Invalid handle")
    }
}
