
use crate::config::{USER_SPACE_END_EXCLUSIVE, USER_SPACE_START};
use alloc::string::String;
use alloc::vec;

/// Validates that `[ptr, ptr + len)` is a canonical lower-half userspace range.
pub fn validate_user_range(ptr: *const u8, len: usize) -> Result<(), &'static str> {
    let start = ptr as usize;

    if len == 0 {
        return Ok(());
    }

    if start < USER_SPACE_START {
        return Err("Pointer below userspace base");
    }

    let end = start
        .checked_add(len)
        .ok_or("Pointer range overflow")?;

    if end > USER_SPACE_END_EXCLUSIVE {
        return Err("Pointer outside userspace range");
    }

    Ok(())
}

/// Copies bytes from userspace into a kernel buffer.
///
/// This helper validates the address range before dereferencing the source
/// pointer to avoid faults caused by obvious kernel/invalid addresses.
pub fn copy_from_user(dst: &mut [u8], src_user: *const u8) -> Result<(), &'static str> {
    validate_user_range(src_user, dst.len())?;

    if dst.is_empty() {
        return Ok(());
    }

    // SAFETY: The userspace range is validated and destination is a valid slice.
    unsafe {
        core::ptr::copy_nonoverlapping(src_user, dst.as_mut_ptr(), dst.len());
    }

    Ok(())
}

/// Copies bytes from a kernel buffer into userspace.
///
/// This helper validates the destination userspace range before writing.
pub fn copy_to_user(dst_user: *mut u8, src: &[u8]) -> Result<(), &'static str> {
    validate_user_range(dst_user as *const u8, src.len())?;

    if src.is_empty() {
        return Ok(());
    }

    // SAFETY: The userspace destination range is validated and source is a valid slice.
    unsafe {
        core::ptr::copy_nonoverlapping(src.as_ptr(), dst_user, src.len());
    }

    Ok(())
}

/// Copies UTF-8 text from userspace into a kernel `String`.
///
/// `requested_len` is clamped to `max_len` to keep copy sizes bounded.
pub fn copy_utf8_from_user(
    src_user: *const u8,
    requested_len: usize,
    max_len: usize,
) -> Result<String, &'static str> {
    let len = requested_len.min(max_len);
    let mut buf = vec![0u8; len];
    copy_from_user(&mut buf, src_user)?;
    core::str::from_utf8(&buf)
        .map(String::from)
        .map_err(|_| "Invalid UTF-8")
}
