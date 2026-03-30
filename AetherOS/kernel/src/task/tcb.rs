#![allow(dead_code)] // Allow dead code for now as not all functions might be used immediately

extern crate alloc;
use alloc::string::String;
use alloc::vec::Vec;
use x86_64::VirtAddr;

use crate::caps::Capability;

pub type TaskId = u64;

/// Represents the possible states of a task.
#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum TaskState {
    Running,
    Ready,
    Blocked,
    Exited,
}

impl Default for TaskState {
    fn default() -> Self {
        Self::Ready
    }
}

/// Minimal callee-saved register snapshot used for context switching.
#[derive(Debug, Clone, Copy, Default)]
#[repr(C)]
pub struct Registers {
    pub rbx: u64,
    pub rbp: u64,
    pub r12: u64,
    pub r13: u64,
    pub r14: u64,
    pub r15: u64,
    pub rsp: u64,
    pub rip: u64,
    pub rflags: u64,
}

/// Backward-compatible name used across the existing scheduler code.
pub type Context = Registers;

/// Minimal scheduler-facing task snapshot.
#[derive(Debug, Clone, Copy)]
pub struct Task {
    pub id: TaskId,
    pub stack_ptr: *mut u8,
    pub registers: Registers,
    pub state: TaskState,
}

impl Default for Task {
    fn default() -> Self {
        Self {
            id: 0,
            stack_ptr: core::ptr::null_mut(),
            registers: Registers::default(),
            state: TaskState::default(),
        }
    }
}

/// A simplified Task Control Block (TCB) for a V-Node or kernel thread.
#[derive(Debug, Clone)] // Derive Clone for easier passing around in mocks/stubs
pub struct TaskControlBlock {
    pub id: u64,
    pub name: String,
    pub state: TaskState,
    pub context: Context,
    pub capabilities: Vec<Capability>,
    pub kernel_stack_base: Option<VirtAddr>,
    pub user_stack_base: Option<VirtAddr>,
    pub address_space_pages: Vec<VirtAddr>,
    pub address_space_root: u64,
}

impl TaskControlBlock {
    /// Creates a new TaskControlBlock with the given parameters.
    pub fn new(id: u64, name: String, capabilities: Vec<Capability>) -> Self {
        Self {
            id,
            name,
            state: TaskState::Ready, // New tasks start in the Ready state
            context: Context::default(),
            capabilities,
            kernel_stack_base: None,
            user_stack_base: None,
            address_space_pages: Vec::new(),
            address_space_root: crate::arch::x86_64::paging::get_kernel_pml4(),
        }
    }

    /// Creates a task with explicit stack and address-space mappings.
    pub fn with_memory_layout(
        id: u64,
        name: String,
        capabilities: Vec<Capability>,
        context: Context,
        kernel_stack_base: Option<VirtAddr>,
        user_stack_base: Option<VirtAddr>,
        address_space_pages: Vec<VirtAddr>,
        address_space_root: u64,
    ) -> Self {
        Self {
            id,
            name,
            state: TaskState::Ready,
            context,
            capabilities,
            kernel_stack_base,
            user_stack_base,
            address_space_pages,
            address_space_root,
        }
    }

    /// Creates a runnable user task with an initialized first-run CPU context.
    pub fn new_user_task(
        id: u64,
        name: String,
        capabilities: Vec<Capability>,
        entry_point: VirtAddr,
        stack_top: VirtAddr,
        address_space_root: u64,
    ) -> Self {
        let context = Context {
            rip: entry_point.as_u64(),
            rsp: stack_top.as_u64(),
            rflags: 0x202, // IF=1, reserved bit set
            ..Context::default()
        };
        let user_stack_base = VirtAddr::new(stack_top.as_u64().saturating_sub(4096));

        Self {
            id,
            name,
            state: TaskState::Ready,
            context,
            capabilities,
            kernel_stack_base: None,
            user_stack_base: Some(user_stack_base),
            address_space_pages: Vec::new(),
            address_space_root,
        }
    }
}
