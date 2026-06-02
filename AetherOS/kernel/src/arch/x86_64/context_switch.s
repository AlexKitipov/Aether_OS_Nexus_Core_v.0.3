// x86_64 callee-saved task context switch.
//
// System V AMD64 ABI arguments:
//   rdi = old TaskContext* (save current CPU context here)
//   rsi = new TaskContext* (restore CPU context from here)
//
// Keep these offsets synchronized with `TaskContext` in
// `kernel/src/task/tcb.rs`; Rust compile-time assertions validate them.
.intel_syntax noprefix
.global context_switch
.type context_switch, @function
context_switch:
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
.size context_switch, . - context_switch
