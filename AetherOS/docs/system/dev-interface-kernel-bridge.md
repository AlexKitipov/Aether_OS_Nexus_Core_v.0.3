# Developer Interface ↔ Kernel Bridge (v0.4 prototype)

This document defines the initial integration layer between a lightweight Replit-style development interface and the AetherOS Nexus kernel.

## 1) Kernel entry points for external interaction

The current kernel already exposes three interaction surfaces that are useful for a dev interface:

- **Syscall layer (`kernel/src/syscall.rs`)**
  - `SYS_IPC_SEND` / `SYS_IPC_RECV` for generic mailbox transport.
  - Domain routing syscalls (`SYS_UI_CALL`, `SYS_VFS_CALL`, `SYS_AI_CALL`, `SYS_SWARM_CALL`) for channel-scoped service calls.
- **Mailbox IPC (`kernel/src/ipc/mailbox.rs`)**
  - In-kernel channel queues with waiter wakeups.
  - Extended with `ensure_channel()` so a stable well-known channel can exist before any user-space service starts.
- **Scheduler/runtime inspection (`kernel/src/task/scheduler.rs`, `kernel/src/runtime.rs`)**
  - Current task id, task map, runnable queue, timer ticks, memory pressure data.

## 2) IPC/message-passing contract for the dev interface

A dedicated IPC contract now lives in `common/src/ipc/dev_interface_ipc.rs`.

### Request types

- `Ping`
- `RunTask { name, inherit_from, executable_path }`
- `InspectSystemState`
- `InspectVNodeCapabilities { task_id }`

### Response types

- `Pong`
- `TaskStarted { task_id, name, mode }`
- `SystemState { ticks, current_task_id, task_count, runnable_tasks, mem_used, mem_free }`
- `VNodeCapabilities { task_id, capabilities, device_rights }`
- `Error { message }`

The transport uses `postcard` serialization and the well-known channel id `well_known::DEV_INTERFACE` (`0x010F`).

## 3) Minimal command pipeline

The request/response cycle is now:

1. Interface serializes `DevInterfaceRequest` and sends it to channel `0x010F`.
2. Kernel bridge (`kernel/src/dev_interface.rs`) polls the channel and handles one request.
3. Kernel serializes `DevInterfaceResponse` and writes it back to the same channel.
4. Interface receives and decodes the response.

This pipeline is intentionally minimal and synchronous-at-edge, while still using the asynchronous mailbox internals.

## 4) Connected operations

The initial bridge supports:

- **Running a task**
  - Inherit capabilities from an existing task and create a new runnable task.
  - Optionally spawn from AetherFS path with inherited capabilities.
- **Inspecting system state**
  - Returns timer ticks, scheduler summary, and memory usage snapshot.
- **Inspecting V-Node/capability context**
  - Returns task capability labels and computed device rights (`read/write/manage/interrupt`) for the target task.

## 5) Modularity and compatibility strategy

To keep kernel evolution decoupled from the interface:

- The bridge is isolated in `kernel/src/dev_interface.rs`.
- Message schema is centralized in `aetheros_common`, allowing user-space tooling and kernel to share one typed protocol.
- Well-known channel constant (`DEV_INTERFACE`) is versionable and can be migrated without changing scheduler/device internals.
- The bridge is polled from the main loop, so it can later move to a dedicated service task without changing the IPC contract.

## 6) Expansion notes

Future upgrades can add:

- Multi-part responses (streaming logs/stdout for long-running tasks).
- Request correlation ids and explicit response envelopes.
- Capability-gated command classes (debug vs admin vs automation roles).
- Pluggable adapters (WebSocket/HTTP bridge in host tools) that only translate transport, not command semantics.
