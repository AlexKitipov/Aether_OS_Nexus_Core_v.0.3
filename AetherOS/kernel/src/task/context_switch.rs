//! Low-level task context switching primitives.

use crate::task::tcb::Registers;

const _: () = {
    use core::mem::{offset_of, size_of};

    assert!(size_of::<Registers>() == 0x48);
    assert!(offset_of!(Registers, rbx) == 0x00);
    assert!(offset_of!(Registers, rbp) == 0x08);
    assert!(offset_of!(Registers, r12) == 0x10);
    assert!(offset_of!(Registers, r13) == 0x18);
    assert!(offset_of!(Registers, r14) == 0x20);
    assert!(offset_of!(Registers, r15) == 0x28);
    assert!(offset_of!(Registers, rsp) == 0x30);
    assert!(offset_of!(Registers, rip) == 0x38);
    assert!(offset_of!(Registers, rflags) == 0x40);
};

#[cfg(all(target_arch = "x86_64", target_os = "none"))]
core::arch::global_asm!(include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/src/arch/x86_64/context_switch.s"
)));

#[cfg(all(target_arch = "x86_64", target_os = "none"))]
unsafe extern "C" {
    fn context_switch(old: *mut Registers, new: *const Registers);
}

/// Switches execution from `old` to `new` register snapshots.
#[inline]
pub unsafe fn switch(old: &mut Registers, new: &Registers) {
    #[cfg(all(target_arch = "x86_64", target_os = "none"))]
    {
        // SAFETY: caller guarantees both pointers are valid context snapshots.
        context_switch(old as *mut Registers, new as *const Registers);
    }

    #[cfg(not(all(target_arch = "x86_64", target_os = "none")))]
    {
        // Host-mode fallback used by tests/tools: model a handoff by copying registers.
        *old = *new;
    }
}

/// Transfers from kernel to user context on first task entry.
#[cfg(all(target_arch = "x86_64", target_os = "none"))]
pub unsafe fn enter_user_mode(entry: u64, user_stack: u64, rflags: u64) -> ! {
    // SAFETY: Caller guarantees:
    // - `entry` is a canonical user-mode RIP mapped executable.
    // - `user_stack` is a canonical user-mode writable stack top.
    // - `rflags` has architecturally required bits (e.g., bit 1) and desired IF state.
    // We load the kernel RSP for the iret frame construction, then execute `iretq`
    // to atomically transition privilege levels using fixed user segments.
    core::arch::asm!(
        "mov rsp, {stack}",
        "push 0x23",          // user data segment selector
        "push {stack}",
        "push {rflags}",
        "push 0x1b",          // user code segment selector
        "push {entry}",
        "iretq",
        stack = in(reg) user_stack,
        rflags = in(reg) rflags,
        entry = in(reg) entry,
        options(noreturn)
    );
}
