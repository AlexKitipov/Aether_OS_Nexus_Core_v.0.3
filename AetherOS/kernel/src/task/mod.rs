//! Task subsystem module declarations and facade helpers.

extern crate alloc;

use alloc::string::String;
use alloc::vec::Vec;
use x86_64::VirtAddr;

use crate::caps::Capability;
use crate::memory::address_space::{self, UserSegment, UserSegmentFlags};

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

/// Creates a new task with a custom scheduler quantum in timer ticks.
pub fn create_task_with_timeslice(
    id: u64,
    name: &str,
    capabilities: Vec<Capability>,
    timeslice_ticks: u64,
) {
    let tcb = TaskControlBlock::new_with_timeslice(
        id,
        String::from(name),
        capabilities,
        timeslice_ticks,
    );
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

    let entry_point = VirtAddr::new(address_space::USER_CODE_BASE);
    let segment = UserSegment {
        virtual_start: entry_point,
        bytes: &code,
        flags: UserSegmentFlags::EXECUTABLE,
    };
    let layout = address_space::create_vnode_address_space(
        &[segment],
        address_space::DEFAULT_USER_STACK_PAGES,
    )
    .map_err(String::from)?;

    let mut tcb = TaskControlBlock::new_user_task(
        id,
        String::from(name),
        capabilities,
        entry_point,
        layout.user_stack_top,
        layout.root_pml4(),
    );
    tcb.user_stack_base = Some(layout.user_stack_base);
    tcb.set_address_space_layout(layout.mapped_pages, layout.owned_frames, layout.root_pml4());
    scheduler::add_task(tcb);

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
pub fn block_current_on_channel(channel_id: u32) {
    let current_task_id = scheduler::get_current_task_id();
    let _ = crate::ipc::mailbox::register_receiver_waiter(channel_id, current_task_id);
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

/// Timer integration hook for preemptive scheduling.
pub fn on_timer_tick() {
    scheduler::on_timer_tick();
}
