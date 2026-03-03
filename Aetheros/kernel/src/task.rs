// kernel/src/task.rs

#![allow(dead_code)]

extern crate alloc;

use alloc::vec::Vec;
use alloc::string::String;
use crate::caps::Capability;
use crate::task::tcb::{TaskControlBlock, TaskState};
use crate::task::scheduler;

// Re-export TaskState and Capability for convenience if needed by external modules
pub use crate::task::tcb::TaskState;
pub use crate::caps::Capability;

/// Initializes the task management system, which includes the scheduler.
pub fn init() {
    scheduler::init();
}

/// Creates a new task and adds it to the scheduler.
pub fn create_task(id: u64, name: &str, capabilities: Vec<Capability>) {
    let tcb = TaskControlBlock::new(id, String::from(name), capabilities);
    scheduler::add_task(tcb);
}

/// Returns a clone of the currently executing task's TaskControlBlock.
pub fn get_current_task() -> TaskControlBlock {
    scheduler::get_current_task_tcb()
}

/// Blocks the current task on an IPC channel.
pub fn block_current_on_channel(channel_id: u32) {
    // In a real IPC implementation, the channel ID would be associated with the task
    // and used by `ipc::kernel_send` to unblock.
    // For now, this just marks the task as blocked and triggers a schedule.
    scheduler::block_current_task();
    // The IPC module will directly unblock by calling `scheduler::unblock_task`.
}

/// Unblocks a task that was waiting on a specific IPC channel.
pub fn unblock_task_on_channel(task_id: u64) {
    scheduler::unblock_task(task_id);
}

/// Explicitly yields CPU to another task.
pub fn schedule() {
    scheduler::schedule();
}
