#![allow(dead_code)] // Allow dead code for now as not all functions might be used immediately

extern crate alloc;
use alloc::string::String;
use alloc::vec::Vec;
use x86_64::VirtAddr;

use crate::caps::Capability;

/// Represents the possible states of a task.
#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum TaskState {
    Running,
    Ready,
    Blocked,
    Exited,
}

/// A simplified Task Control Block (TCB) for a V-Node or kernel thread.
/// In a real microkernel, this would hold much more state (registers, memory map, capabilities).
/// For initial implementation, focus on `id`, `name`, `state`, and `capabilities` as placeholders.
#[derive(Debug, Clone)] // Derive Clone for easier passing around in mocks/stubs
pub struct TaskControlBlock {
    pub id: u64,
    pub name: String,
    pub state: TaskState,
    pub capabilities: Vec<Capability>,
    pub kernel_stack_base: Option<VirtAddr>,
    pub user_stack_base: Option<VirtAddr>,
    pub address_space_pages: Vec<VirtAddr>,
}

impl TaskControlBlock {
    /// Creates a new TaskControlBlock with the given parameters.
    pub fn new(id: u64, name: String, capabilities: Vec<Capability>) -> Self {
        Self {
            id,
            name,
            state: TaskState::Ready, // New tasks start in the Ready state
            capabilities,
            kernel_stack_base: None,
            user_stack_base: None,
            address_space_pages: Vec::new(),
        }
    }

    /// Creates a task with explicit stack and address-space mappings.
    pub fn with_memory_layout(
        id: u64,
        name: String,
        capabilities: Vec<Capability>,
        kernel_stack_base: Option<VirtAddr>,
        user_stack_base: Option<VirtAddr>,
        address_space_pages: Vec<VirtAddr>,
    ) -> Self {
        Self {
            id,
            name,
            state: TaskState::Ready,
            capabilities,
            kernel_stack_base,
            user_stack_base,
            address_space_pages,
        }
    }
}
