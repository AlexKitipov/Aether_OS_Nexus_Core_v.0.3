//! Task subsystem module declarations and facade helpers.

extern crate alloc;

use alloc::string::String;
use alloc::vec::Vec;

use crate::caps::Capability;

pub mod scheduler;
pub mod tcb;

pub use tcb::{TaskControlBlock, TaskState};

/// Initializes the task management system, which includes the scheduler.
pub fn init() {
    scheduler::init();
}

/// Creates a new task and adds it to the scheduler.
pub fn create_task(id: u64, name: &str, capabilities: Vec<Capability>) {
    let tcb = TaskControlBlock::new(id, String::from(name), capabilities);
    scheduler::add_task(tcb);
}

/// Returns a clone of the currently executing task's TCB.
pub fn get_current_task() -> TaskControlBlock {
    scheduler::get_current_task_tcb()
}

/// Blocks the current task on an IPC channel.
pub fn block_current_on_channel(_channel_id: u32) {
    scheduler::block_current_task();
}

/// Unblocks a task that was waiting on a specific IPC channel.
pub fn unblock_task_on_channel(task_id: u64) {
    scheduler::unblock_task(task_id);
}

/// Explicitly yields CPU to another task.
pub fn schedule() {
    scheduler::schedule();
}
