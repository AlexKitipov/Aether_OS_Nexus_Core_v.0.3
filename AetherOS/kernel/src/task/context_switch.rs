//! Low-level task context switching primitives.

use crate::task::tcb::Registers;

#[cfg(all(target_arch = "x86_64", target_os = "none"))]
core::arch::global_asm!(
    r#"
.global context_switch
context_switch:
    // rdi = old, rsi = new
    mov [rdi + 0x00], rbx
    mov [rdi + 0x08], rbp
    mov [rdi + 0x10], r12
    mov [rdi + 0x18], r13
    mov [rdi + 0x20], r14
    mov [rdi + 0x28], r15
    mov [rdi + 0x30], rsp

    lea rax, [rip + .Lresume]
    mov [rdi + 0x38], rax
    pushfq
    pop qword ptr [rdi + 0x40]

    mov rbx, [rsi + 0x00]
    mov rbp, [rsi + 0x08]
    mov r12, [rsi + 0x10]
    mov r13, [rsi + 0x18]
    mov r14, [rsi + 0x20]
    mov r15, [rsi + 0x28]
    mov rsp, [rsi + 0x30]

    push qword ptr [rsi + 0x40]
    popfq
    jmp [rsi + 0x38]

.Lresume:
    ret
"#
);

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
