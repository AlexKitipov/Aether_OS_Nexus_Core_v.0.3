//! Kernel virtual address-space boundaries.
//!
//! These constants define a conservative lower-half userspace region used by
//! `usercopy` and fault classification logic.

/// Inclusive lower bound of userspace virtual memory.
pub const USER_SPACE_START: usize = 0x0000_0000_0000_1000;

/// Exclusive upper bound of userspace virtual memory.
pub const USER_SPACE_END_EXCLUSIVE: usize = 0x0000_8000_0000_0000;
