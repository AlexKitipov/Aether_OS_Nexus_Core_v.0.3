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

/// Saved CPU context for preemptive scheduling.
///
/// This is intentionally architecture-neutral scaffolding. In a fully wired
/// x86_64 context switch, these fields would map to the exact interrupt frame
/// and callee/caller-saved registers.
#[derive(Debug, Clone, Copy)]
pub struct TaskContext {
    pub rip: u64,
    pub rsp: u64,
    pub rflags: u64,
    pub regs: [u64; 16],
}

impl TaskContext {
    pub const fn new() -> Self {
        Self {
            rip: 0,
            rsp: 0,
            rflags: 0x202,
            regs: [0; 16],
        }
    }
}

/// A Task Control Block (TCB) for a V-Node or kernel thread.
#[derive(Debug, Clone)]
pub struct TaskControlBlock {
    pub id: u64,
    pub name: String,
    pub state: TaskState,
    pub capabilities: Vec<Capability>,
    pub context: TaskContext,
    pub timeslice_ticks: u64,
    pub consumed_ticks: u64,
    pub switch_count: u64,
}

impl TaskControlBlock {
    /// Creates a new TaskControlBlock with the given parameters.
    pub fn new(id: u64, name: String, capabilities: Vec<Capability>) -> Self {
        Self {
            id,
            name,
            state: TaskState::Ready, // New tasks start in the Ready state
            capabilities,
            context: TaskContext::new(),
            timeslice_ticks: 5,
            consumed_ticks: 0,
            switch_count: 0,
        }
    }

    pub fn with_timeslice(mut self, ticks: u64) -> Self {
        self.timeslice_ticks = ticks.max(1);
        self
    }

    pub fn reset_timeslice(&mut self) {
        self.consumed_ticks = 0;
    }
}
