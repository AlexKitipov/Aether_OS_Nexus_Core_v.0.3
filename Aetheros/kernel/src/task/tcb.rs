#![allow(dead_code)] // Allow dead code for now as not all functions might be used immediately

extern crate alloc;
use alloc::string::String;
use alloc::vec::Vec;

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
    // pub stack_pointer: usize, // Conceptual for context switching
    // pub cpu_state: CpuState, // Conceptual for saving registers
}

impl TaskControlBlock {
    /// Creates a new TaskControlBlock with the given parameters.
    pub fn new(id: u64, name: String, capabilities: Vec<Capability>) -> Self {
        Self {
            id,
            name,
            state: TaskState::Ready, // New tasks start in the Ready state
            capabilities,
        }
    }
}

