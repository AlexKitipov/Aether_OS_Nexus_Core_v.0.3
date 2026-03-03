//! Central kernel configuration constants.

/// Maximum number of IPC channels exposed by the kernel syscall ABI.
pub const IPC_CHANNEL_COUNT: u32 = 32;

/// AetherOS page size in bytes.
pub const PAGE_SIZE: usize = 4096;

/// Reserved conceptual kernel memory size in bytes (256 MiB).
pub const KERNEL_MEMORY_SIZE: usize = 256 * 1024 * 1024;
