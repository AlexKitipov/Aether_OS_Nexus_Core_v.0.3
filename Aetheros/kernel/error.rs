//! # Kernel Error Types
//!
//! This module defines all possible error conditions in the AetherOS kernel,
//! providing type-safe error handling instead of generic Result<T, ()>.

/// Strongly typed kernel errors with detailed context.
///
/// # Examples
///
/// ```
/// use kernel::error::{KernelError, Result};
///
/// fn allocate_channel(id: u32) -> Result<()> {
///     if id >= 32 {
///         return Err(KernelError::InvalidChannelId(id));
///     }
///     Ok(())
/// }
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KernelError {
    /// Invalid IPC channel identifier.
    ///
    /// The provided channel ID is outside the valid range (0..IPC_CHANNEL_COUNT).
    InvalidChannelId(u32),

    /// Output buffer too small for received message.
    ///
    /// Contains the required and provided buffer sizes in bytes.
    BufferTooSmall { required: usize, provided: usize },

    /// Permission denied for this operation.
    ///
    /// The current task lacks required capability.
    PermissionDenied,

    /// Out of memory for allocation.
    OutOfMemory,

    /// Invalid file descriptor.
    InvalidFd,

    /// Invalid argument passed to function.
    InvalidArgument(&'static str),

    /// Operation would block (non-blocking mode).
    WouldBlock,

    /// Device or resource busy.
    Busy,
}

impl KernelError {
    /// Convert error to syscall return code.
    ///
    /// # Examples
    /// ```
    /// assert_eq!(KernelError::PermissionDenied.to_syscall_code(), 0xFFFFFFFFFFFFFFFE);
    /// ```
    pub fn to_syscall_code(self) -> u64 {
        match self {
            Self::PermissionDenied => crate::kernel::syscall::E_ACC_DENIED,
            _ => crate::kernel::syscall::E_ERROR,
        }
    }
}

impl core::fmt::Display for KernelError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::InvalidChannelId(id) => write!(f, "Invalid channel ID: {}", id),
            Self::BufferTooSmall { required, provided } => {
                write!(f, "Buffer too small: need {}, got {}", required, provided)
            }
            Self::PermissionDenied => write!(f, "Permission denied"),
            Self::OutOfMemory => write!(f, "Out of memory"),
            Self::InvalidFd => write!(f, "Invalid file descriptor"),
            Self::InvalidArgument(msg) => write!(f, "Invalid argument: {}", msg),
            Self::WouldBlock => write!(f, "Operation would block"),
            Self::Busy => write!(f, "Device or resource busy"),
        }
    }
}

/// Specialized `Result` type for kernel operations.
pub type Result<T> = core::result::Result<T, KernelError>;
