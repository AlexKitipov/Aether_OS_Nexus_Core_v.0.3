// kernel/src/task.rs

#![allow(dead_code)]

extern crate alloc;

use alloc::string::String;
use alloc::vec::Vec;

use crate::caps::Capability;
use crate::task::scheduler;
use crate::task::tcb::TaskControlBlock;

// Re-export for convenience.
pub use crate::caps::Capability as TaskCapability;
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

/// Creates a new task with an explicit timeslice in timer ticks.
pub fn create_task_with_timeslice(id: u64, name: &str, capabilities: Vec<Capability>, timeslice_ticks: u64) {
    let tcb = TaskControlBlock::new(id, String::from(name), capabilities).with_timeslice(timeslice_ticks);
    scheduler::add_task(tcb);
}

/// Returns a clone of the currently executing task's TaskControlBlock.
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
    scheduler::yield_current_task();
}

/// Called by timer interrupt flow to enforce preemption.
pub fn on_timer_tick() {
    scheduler::on_timer_tick();
}
