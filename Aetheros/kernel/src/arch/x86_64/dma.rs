// kernel/src/arch/x86_64/dma.rs

#![allow(dead_code)] // Allow dead code for now as not all functions might be used immediately

extern crate alloc;
use alloc::collections::BTreeMap;
use alloc::vec::Vec;
use core::sync::atomic::{AtomicU64, Ordering};
use spin::Mutex;
use crate::kprintln;

/// A simple DMA buffer manager for simulation.
/// In a real system, this would manage physically contiguous memory pages
/// and provide their physical addresses to devices.
/// For V-Nodes, these buffers are mapped into their virtual address space.

/// Static counter for generating unique DMA buffer handles.
static NEXT_HANDLE: AtomicU64 = AtomicU64::new(1);

/// Stores the allocated DMA buffers, mapped by their unique handles.
/// The `Vec<u8>` acts as the memory backing for the DMA buffer.
static DMA_BUFFERS: Mutex<BTreeMap<u64, Vec<u8>>> = Mutex::new(BTreeMap::new());

/// Allocates a new DMA-capable buffer of the specified `size`.
/// Returns a unique handle to the buffer, or `None` if allocation fails.
///
/// In a real system, this would involve allocating physically contiguous memory.
pub fn alloc_dma_buffer(size: usize) -> Option<u64> {
    let handle = NEXT_HANDLE.fetch_add(1, Ordering::SeqCst);
    let mut buffers = DMA_BUFFERS.lock();

    // Allocate a Vec with the given capacity. This simulates a contiguous memory block.
    let buffer = Vec::with_capacity(size);
    buffers.insert(handle, buffer);

    kprintln!("[kernel] dma: Allocated buffer with handle {} and size {}.", handle, size);
    Some(handle)
}

/// Frees the DMA buffer associated with the given `handle`.
pub fn free_dma_buffer(handle: u64) {
    let mut buffers = DMA_BUFFERS.lock();
    if buffers.remove(&handle).is_some() {
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
    buffers.get_mut(&handle).map(|buf| buf.as_mut_ptr())
}

/// Returns the current capacity (allocated size) of the DMA buffer.
pub fn get_dma_buffer_capacity(handle: u64) -> Option<usize> {
    let buffers = DMA_BUFFERS.lock();
    buffers.get(&handle).map(|buf| buf.capacity())
}

/// Sets the effective length of the data within the DMA buffer.
/// This is used to indicate how much of the buffer is currently valid data.
pub fn set_dma_buffer_len(handle: u64, len: usize) -> Result<(), &'static str> {
    let mut buffers = DMA_BUFFERS.lock();
    if let Some(buf) = buffers.get_mut(&handle) {
        if len <= buf.capacity() {
            // SAFETY: We checked `len <= capacity`, so this is safe.
            // This is crucial for `Vec` to function correctly as a buffer.
            unsafe { buf.set_len(len); }
            kprintln!("[kernel] dma: Set length for handle {} to {}.", handle, len);
            Ok(())
        } else {
            kprintln!("[kernel] dma: Error setting length for handle {}: {} exceeds capacity {}.", handle, len, buf.capacity());
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
    buffers.get(&handle).map(|buf| buf.len())
}
