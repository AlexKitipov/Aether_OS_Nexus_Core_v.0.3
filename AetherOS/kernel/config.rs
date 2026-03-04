//! Central kernel configuration constants.

/// Maximum number of IPC channels exposed by the kernel syscall ABI.
pub const IPC_CHANNEL_COUNT: u32 = 32;

/// AetherOS page size in bytes.
pub const PAGE_SIZE: usize = 4096;

/// Reserved conceptual kernel memory size in bytes (256 MiB).
pub const KERNEL_MEMORY_SIZE: usize = 256 * 1024 * 1024;

/// Lowest canonical userspace address accepted by kernel copy helpers.
pub const USER_SPACE_START: usize = 0x1000;

/// End-exclusive bound of canonical lower-half userspace addresses.
pub const USER_SPACE_END_EXCLUSIVE: usize = 0x0000_8000_0000_0000;
