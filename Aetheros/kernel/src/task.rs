// kernel/src/task.rs

#![allow(dead_code)]

extern crate alloc;

use alloc::string::String;
use alloc::vec::Vec;
use crate::task::scheduler;
use crate::task::tcb::TaskControlBlock;

pub use crate::caps::Capability;
pub use crate::task::tcb::TaskState;

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

/// Blocks the current task on an IPC channel and records wait intent with the IPC subsystem.
pub fn block_current_on_channel(channel_id: u32) {
    let current_id = get_current_task().id;
    let _ = crate::ipc::kernel_register_receiver_waiter(channel_id, current_id);
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
