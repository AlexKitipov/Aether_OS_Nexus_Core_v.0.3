//! Task subsystem module declarations and facade helpers.

extern crate alloc;

use alloc::string::String;
use alloc::vec::Vec;
use x86_64::VirtAddr;

use crate::caps::Capability;
use crate::memory::page_allocator::PageAllocator;

pub mod context_switch;
pub mod scheduler;
pub mod tcb;

pub use tcb::{Context, TaskControlBlock, TaskState};

/// Initializes the task management system, which includes the scheduler.
pub fn init() {
    scheduler::init();
}

/// Creates a new task and adds it to the scheduler.
pub fn create_task(id: u64, name: &str, capabilities: Vec<Capability>) {
    let tcb = TaskControlBlock::new(id, String::from(name), capabilities);
    scheduler::add_task(tcb);
}

/// Creates a new task inheriting all capabilities from an existing task.
pub fn create_task_inheriting(parent_task_id: u64, id: u64, name: &str) -> bool {
    let parent = match scheduler::get_task(parent_task_id) {
        Some(task) => task,
        None => return false,
    };

    let tcb = TaskControlBlock::new(id, String::from(name), parent.capabilities);
    scheduler::add_task(tcb);
    true
}

/// Creates a user task with first-run context initialized for entry and stack.
pub fn create_user_task(
    id: u64,
    name: &str,
    capabilities: Vec<Capability>,
    entry_point: VirtAddr,
    stack_top: VirtAddr,
    address_space_root: u64,
) {
    let tcb = TaskControlBlock::new_user_task(
        id,
        String::from(name),
        capabilities,
        entry_point,
        stack_top,
        address_space_root,
    );
    scheduler::add_task(tcb);
}

/// Loads a conceptual binary from AetherFS and spawns a runnable user task.
///
/// This helper ties together three subsystems:
/// - AetherFS (binary lookup)
/// - Paging/Page allocator (user stack allocation)
/// - Scheduler (task registration)
pub fn spawn_from_file(path: &str, id: u64, name: &str, capabilities: Vec<Capability>) -> Result<(), String> {
    let code = crate::aetherfs::read_file(path)?;
    if code.is_empty() {
        return Err(String::from("Refusing to spawn empty binary"));
    }

    // Conceptual fixed user mapping layout until ELF segment mapping lands.
    let entry_point = VirtAddr::new(0x0000_0000_4000_0000);
    let stack_base = PageAllocator::allocate_page()
        .ok_or_else(|| String::from("Failed to allocate user stack page"))?;
    let stack_top = stack_base + 4096u64;

    let address_space_root = crate::arch::x86_64::paging::get_kernel_pml4();
    create_user_task(
        id,
        name,
        capabilities,
        entry_point,
        stack_top,
        address_space_root,
    );

    crate::kprintln!(
        "[kernel] task: Spawned task {} from {} ({} bytes)",
        id,
        path,
        code.len()
    );
    Ok(())
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

/// Bootstraps the first dynamic userspace-like task after heap initialization.
///
/// This task is intentionally minimal and exists to validate that heap-backed
/// structures (`String`, `Vec`) and scheduler registration are functional.
pub fn bootstrap_first_dynamic_task() -> bool {
    let task_id = 1;
    if scheduler::get_task(task_id).is_some() {
        return false;
    }

    let capabilities = alloc::vec![
        Capability::LogWrite,
        Capability::TimeRead,
    ];

    create_task(task_id, "init.dynamic", capabilities);
    crate::kprintln!(
        "[kernel] task: Bootstrapped first dynamic task '{}' (ID: {}).",
        "init.dynamic",
        task_id
    );
    true
}

/// Explicitly yields CPU to another task.
pub fn schedule() {
    scheduler::schedule();
}

/// Saves CPU register snapshot for the currently running task.
pub fn save_current_context(snapshot: Context) {
    scheduler::save_current_context(snapshot);
}
