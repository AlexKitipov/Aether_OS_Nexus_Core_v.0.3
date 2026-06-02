# AetherOS Nexus Core v0.3 — Kernel Bootability and Subsystem Audit

Date: 2026-06-02

This audit treats the repository as a real Rust/x86_64 operating-system kernel and focuses on concrete code-level blockers, incomplete subsystems, and PR-ready remediation work. It intentionally avoids inventing modules that are not present.

## Commands used

- `find .. -name AGENTS.md -print`
- `rg --files -g '!target' -g '!build'`
- `find . -maxdepth 4 \( -name 'linker.ld' -o -name 'bootimage.toml' -o -name 'x86_64-unknown-none.json' -o -name 'context_switch.s' -o -name 'Cargo.toml' -o -name 'build.rs' \) -print | sort`
- `find AetherOS/.cargo AetherOS/kernel/.cargo -maxdepth 2 -type f -print`
- `cargo check -p aetheros-kernel` from `AetherOS/`
- `cargo build-kernel` from `AetherOS/`
- `cargo build --release --target .cargo/aetheros-x86_64.json -Zbuild-std=core,alloc -Zbuild-std-features=compiler-builtins-mem -p aetheros-kernel` from `AetherOS/`
- `cargo build --release --target .cargo/aetheros-x86_64.json -Zbuild-std=core,alloc -Zbuild-std-features=compiler-builtins-mem -p aetheros-kernel --offline` from `AetherOS/`

## Missing files list

| Requested artifact | Status | Existing evidence | Should exist at | Notes |
| --- | --- | --- | --- | --- |
| `linker.ld` | Present, but not wired into target config | `AetherOS/kernel/linker.ld` exists and defines `ENTRY(_start)`, sections, and alignment asserts. | Keep `AetherOS/kernel/linker.ld`; optionally move or symlink to `AetherOS/kernel/src/arch/x86_64/linker.ld` only if build scripts are updated. | The custom target JSON does not pass `-T kernel/linker.ld`, so the script may not be used during the actual bare-metal link. |
| `bootimage.toml` | Missing | No `bootimage.toml` was found. README says legacy `bootimage` is not used. | If adopting bootimage: `AetherOS/bootimage.toml`. If staying on `bootloader_api` 0.11: do not add `bootimage.toml`; add a bootloader image builder instead. | The repository currently mixes bootloader dependency/config language with a direct bare-metal ELF flow. |
| `x86_64-unknown-none.json` | Present | `AetherOS/x86_64-unknown-none.json` exists. Duplicate custom specs also exist as `AetherOS/.cargo/aetheros-x86_64.json` and `AetherOS/kernel/.cargo/aetheros-x86_64.json`. | Prefer one canonical spec at `AetherOS/.cargo/aetheros-x86_64.json` or `AetherOS/x86_64-unknown-none.json`, not three divergent copies. | Current cargo alias uses `.cargo/aetheros-x86_64.json`, not `x86_64-unknown-none.json`. |
| `context_switch.s` | Missing | No standalone `context_switch.s` exists. Inline `global_asm!` exists in `AetherOS/kernel/src/task/context_switch.rs`. | `AetherOS/kernel/src/arch/x86_64/context_switch.s` if the project wants assembly glue as a standalone artifact. | Standalone assembly is useful for ABI review, objdump validation, and keeping arch-specific assembly out of scheduler-facing Rust. |
| Keyboard driver | Partially present | `AetherOS/kernel/src/interrupts/keyboard.rs` reads PS/2 scancodes from port `0x60` and routes them to an IPC channel, but payload serialization is TODO/empty. | If keeping interrupt-first organization: complete `AetherOS/kernel/src/interrupts/keyboard.rs`. If adding driver abstraction: `AetherOS/kernel/src/drivers/keyboard.rs` plus `drivers/mod.rs`. | Current code is an IRQ handler, not a complete keyboard driver. |
| NIC driver | Partially present/stubbed | `AetherOS/kernel/src/drivers/net.rs` defines `VirtIoNetDevice`, a no-op `E1000NetDevice`, and a generic `NetworkDeviceIo`; no PCI/MMIO/virtio queue hardware bring-up exists. | Complete `AetherOS/kernel/src/drivers/net.rs`; optionally split hardware drivers into `AetherOS/kernel/src/drivers/virtio_net.rs` and `AetherOS/kernel/src/drivers/e1000.rs`. | Avoid adding a new file until the chosen NIC model is decided. |
| V-Node address space manager | Missing | V-Node spawning uses `get_kernel_pml4()` as `address_space_root`. | `AetherOS/kernel/src/memory/address_space.rs` or `AetherOS/kernel/src/vnode/address_space.rs`; wire through `memory/mod.rs` and `task/tcb.rs`. | Required for V-Node isolation and for ELF `PT_LOAD` mappings. |
| ELF segment loader implementation | Missing | `ElfLoader::parse_elf_bytes` validates the ELF header and returns header fields only. It does not parse or map program headers. | Extend `AetherOS/kernel/src/elf.rs`. | Add `ProgramHeader` parsing and a minimal `load_segments(...)` API before changing V-Node spawning. |
| Snapshot storage backend | Partially present/incomplete | `SnapshotStorage` exists and `InMemorySnapshotStorage` exists; no persistent block/AetherFS storage backend is present. | `AetherOS/kernel/src/snapshot_engine.rs` for the trait-facing backend, or `AetherOS/kernel/src/aetherfs.rs` if snapshots should be content-addressed in AetherFS. | In-memory storage is lost on reboot and cannot support real restore-from-boot. |

## Missing modules list

1. **Persistent bootloader/image builder module or script**: there is no path that wraps the built kernel ELF into a bootable BIOS/UEFI disk image. `scripts/build_kernel_image.sh` explicitly notes that the artifact is a bare-metal ELF and not directly bootable with `qemu-system-x86_64 -kernel`.
2. **Dedicated V-Node address-space manager**: tasks and V-Nodes currently share the kernel PML4 root instead of receiving isolated page tables.
3. **ELF `PT_LOAD` mapper**: the ELF loader does not map segment virtual addresses, zero BSS tails, enforce segment flags, or validate user-space address ranges.
4. **Real context-dispatch integration**: low-level save/restore assembly exists inline, but the scheduler only logs the chosen context and address space.
5. **Persistent snapshot backend**: only a volatile in-memory backend exists.
6. **Hardware NIC bring-up**: the network driver layer has simulated queues/no-op send/receive paths but no PCI discovery, BAR mapping, virtio descriptor rings, DMA, or IRQ wiring.
7. **Complete keyboard event serialization path**: the IRQ handler creates a `KeyEvent`, but sends an empty payload because postcard serialization is TODO.

## Incomplete subsystems

### Context switching

**What exists**

- `TaskContext`/`Registers` stores callee-saved registers, `rsp`, `rip`, and `rflags`.
- `context_switch.rs` contains inline x86_64 assembly that saves/restores callee-saved registers and jumps to the next saved RIP.
- `enter_user_mode` has an `iretq` skeleton for first user-mode entry.

**What is missing**

- The scheduler does not call `context_switch::switch`; `restore_task_context` only prints `rip`, `rsp`, `rflags`, and address-space root.
- No CR3 switch is performed before dispatching a different address space.
- No kernel stack/TSS update per task is performed before user-mode entry.
- No interrupt/trap-frame based preemption path saves full volatile registers.

**What must be implemented**

- Move or include the assembly as `AetherOS/kernel/src/arch/x86_64/context_switch.s` if a standalone artifact is required.
- Replace `restore_task_context` logging with a safe dispatch path that obtains mutable old/new TCB contexts without holding scheduler locks across the assembly jump.
- Add `switch_address_space(pml4_phys)` in paging code and call it before the low-level switch.
- Add first-run handling for user tasks using `enter_user_mode` only after segment selectors and TSS are correct.

### Timer IRQ to scheduler integration

**What exists**

- PIT setup uses a 100 Hz tick.
- `timer::tick()` increments a global tick counter and calls `task::on_timer_tick()`.
- Scheduler tick accounting sets `RESCHEDULE_REQUESTED` when a task exceeds its quantum.
- The idle loop checks `take_reschedule_request()` and calls `task::schedule()`.

**What is missing**

- No immediate preemptive context switch happens in IRQ context.
- The IRQ handler defers scheduling to the main idle loop, which means a non-yielding kernel task will not be preempted until the loop boundary is reached.
- Full interrupt-frame context capture is missing.

**What must be implemented**

- Keep the current deferred flag as an initial safe milestone.
- Add a later IRQ-exit scheduler path that saves the interrupted task's trap frame and switches on return-from-interrupt.
- Avoid taking scheduler `Mutex` locks inside IRQ paths that can interrupt code already holding those locks.

### APIC/PIC IRQ routing

**What exists**

- PIC remap, IRQ0/IRQ1 unmasking, EOI, and 16 legacy IRQ entries exist.
- IRQ-to-IPC channel mapping exists in `arch/x86_64/irq.rs`.
- A separate `interrupts` module also initializes PIC, timer, and keyboard handlers.

**What is missing**

- No Local APIC or IOAPIC discovery/programming exists.
- There are two overlapping PIC initialization paths: `irq::init()` calls `arch::x86_64::irq::init()`, while `interrupts::init()` has a separate PIC/IDT flow and is not called by top-level init.
- IRQ routing to device/V-Node owners is not consistently integrated with keyboard/timer special handlers.

**What must be implemented**

- Choose one legacy PIC initialization path and remove or delegate the duplicate path.
- Add an explicit `InterruptController` abstraction once APIC support begins.
- Keep PIC-only boot as milestone 1; add IOAPIC only after bootability and scheduler dispatch are stable.

### V-Node address space isolation

**What exists**

- TCBs store `address_space_root` and address-space page ownership metadata.
- V-Node loading records immutable image hashes and capabilities.

**What is missing**

- `spawn_vnode_task` sets `address_space_root` to `get_kernel_pml4()`, so V-Nodes are not isolated.
- There is no user page-table creation, no user stack mapping in a separate PML4, and no user/kernel permission separation per V-Node.
- Page fault handling can terminate user tasks, but the isolation boundary is not actually established.

**What must be implemented**

- Add an address-space manager that allocates a PML4, copies kernel higher-half/direct-map entries, maps user pages with `PRESENT | USER_ACCESSIBLE`, and returns a root frame.
- Track allocated page-table frames and mapped user frames in `TaskControlBlock::address_space_pages`.
- Teach the ELF loader to map each user segment into the new address space.

### ELF program header loading

**What exists**

- `parse_elf_bytes` checks ELF magic, class, endianness, and extracts entry point, program-header offset, and count.

**What is missing**

- No program-header size validation.
- No `PT_LOAD` iteration.
- No virtual-address range validation.
- No segment flag translation to page permissions.
- No copying file bytes into allocated pages and zeroing `memsz - filesz`.

**What must be implemented**

- Add an `ElfProgramHeader` struct for ELF64 fields.
- Validate `e_phentsize == 56`, bounds-check each header, and reject unsupported ABI/type combinations.
- Load only `PT_LOAD` segments into a supplied address space.
- Return an `LoadedElf { entry_point, user_stack_top, address_space_root, mapped_pages }` or equivalent structure used by `vnode_loader`.

### Snapshot storage

**What exists**

- Snapshot structs, postcard encoding, hash wrapping, verification, capture, restore, and an in-memory storage implementation exist.

**What is missing**

- No persistent storage backend.
- Snapshot timestamps are derived from snapshot ID, not wall-clock/RTC/monotonic real time.
- Restore spawns V-Nodes from image hashes but does not restore execution state or address-space contents.

**What must be implemented**

- Add an AetherFS-backed or block-device-backed implementation of `SnapshotStorage`.
- Store snapshot wire bytes content-addressed and atomically update a latest pointer.
- Extend `VNodeState` only after basic V-Node process isolation exists; otherwise restore cannot reconstruct a runnable address space.

### Network stack integration

**What exists**

- Ethernet/IPv4/UDP parsing helpers and UDP capability checks exist.
- `NetworkStack` supports a bound `NetDevice`, but `init()` does not bind any device.
- `VirtIoNetDevice` has test-like RX/TX queues.
- `E1000NetDevice` send/receive are no-ops.

**What is missing**

- No real NIC probe/init path.
- No DMA buffer/ring management.
- No RX polling or IRQ integration.
- `udp_send` only enqueues local loopback data; it does not emit Ethernet/IP/UDP frames to the bound device.
- No network V-Node bridge registration path is wired to the kernel stack.

**What must be implemented**

- Pick one QEMU-friendly NIC first, preferably virtio-net for modern QEMU or e1000 for simpler MMIO/PIO learning.
- Register the selected NIC with the device manager and bind it via `network::with_stack(|s| s.bind_device(...))`.
- Add a `poll_rx()` path that reads frames from `NetDevice`, parses Ethernet/IP/UDP, and enqueues packets.
- Add a `transmit_udp()` path that builds Ethernet/IP/UDP frames and calls `NetDevice::send()`.

## Build system issues

### What is configured

- The repository has a Cargo workspace under `AetherOS/Cargo.toml` with `kernel`, `common`, V-Nodes, UI V-Nodes, and support crates as members.
- The kernel has its own `Cargo.toml`, depends on `bootloader_api = 0.11.15`, and has `build = "build.rs"`.
- A target spec exists at `AetherOS/x86_64-unknown-none.json`, and duplicate specs exist under `.cargo/`.
- `.cargo/config.toml` defines a `build-kernel` alias.
- `rust-toolchain.toml` pins nightly `2025-03-01` and includes `rust-src` and llvm tools.

### Problems

1. **The linker script is not wired into the target JSON or cargo rustflags.** The target spec passes `--gc-sections`, `-nostdlib`, and `-static`, but not `-T kernel/linker.ld`.
2. **The build alias is incomplete for a custom target.** `cargo build-kernel` fails with `can't find crate for core` because it does not pass `-Zbuild-std`.
3. **The bootloader path is absent.** `build.rs` emits placeholder bootloader paths and explicitly says bootimage handles bootloaders, while README says bootimage is not used.
4. **No bootable image output is produced.** `scripts/build_kernel_image.sh` builds a bare-metal ELF and warns it is not directly bootable without a compatible bootloader or UEFI image.
5. **QEMU paths are inconsistent.** `scripts/build_kernel_image.sh` outputs `target/aetheros-x86_64/release/aetheros-kernel`, but `scripts/run_qemu.sh` expects `target/x86_64-unknown-none/release/aetheros-kernel`.
6. **No `bootimage.toml` exists.** That is fine if the repository intentionally avoids bootimage, but the replacement `bootloader_api` 0.11 image-construction flow is not implemented.

### Why the kernel cannot currently produce a bootable image

The repository can host-check the kernel crate, but the bare-metal build path is not enough to boot. A bootable OS image needs a boot protocol and image format that loads the kernel, creates the `BootInfo`, maps framebuffer/memory regions, and jumps to `_start`. The current script builds only a kernel ELF and explicitly warns that it is not directly bootable. Additionally, the custom target build alias omits `-Zbuild-std`, the linker script is not passed to `rust-lld`, and there is no bootloader builder/configuration producing a BIOS disk image, UEFI ESP, or bootloader-wrapped kernel image.

## Critical blockers

1. **No bootloader-wrapped image path**: the kernel cannot boot because the final artifact is only a bare-metal ELF.
2. **Linker script not connected to the actual link command**: section layout exists but is not guaranteed to apply.
3. **Custom target build alias omits `-Zbuild-std`**: bare-metal build fails before linking in this environment.
4. **Scheduler does not perform real context switches**: scheduling decisions are logged, not dispatched.
5. **V-Nodes share the kernel address-space root**: no isolation exists.
6. **ELF loader does not map segments**: V-Node binaries cannot be loaded into user memory.
7. **NIC and keyboard paths are incomplete**: keyboard sends empty payloads; NIC driver is simulated/no-op.
8. **Snapshot storage is volatile only**: no restore after reboot.

## PR-ready fix plan

### PR #1 — Wire the linker script into bare-metal builds

- **File:** `AetherOS/.cargo/aetheros-x86_64.json`
- **File:** `AetherOS/x86_64-unknown-none.json`
- **File:** `AetherOS/kernel/.cargo/aetheros-x86_64.json` if retained
- **Summary:** Add `-Tkernel/linker.ld` or centralize build flags so `rust-lld` uses the existing linker script.
- **Reason:** The kernel has a linker script, but the target spec does not pass it to the linker.
- **Pitfalls:** Relative linker-script paths are resolved from the cargo invocation directory. Prefer invoking from `AetherOS/` and using `kernel/linker.ld`, or switch to an absolute path generated by build tooling.

### PR #2 — Fix the custom target cargo alias

- **File:** `AetherOS/.cargo/config.toml`
- **Summary:** Change `build-kernel` to include `-Zbuild-std=core,alloc,compiler_builtins -Zbuild-std-features=compiler-builtins-mem`.
- **Reason:** Custom `os = "none"` target builds need `core`/`alloc` from source.
- **Pitfalls:** Requires nightly and `rust-src`; offline CI must cache `compiler_builtins` or vendor dependencies.

### PR #3 — Choose and implement one boot image flow

- **File:** `AetherOS/scripts/build_kernel_image.sh`
- **File:** `AetherOS/scripts/run_qemu.sh`
- **File:** `AetherOS/Cargo.toml`
- **Summary:** Either add a `bootloader_api` 0.11 image-builder binary/script or intentionally revert to `bootimage` with `AetherOS/bootimage.toml`.
- **Reason:** Current output is not a bootable BIOS/UEFI image.
- **Pitfalls:** Do not mix `bootimage` 0.10 conventions with `bootloader_api` 0.11 handoff assumptions. Verify the entry ABI and `BootInfo` pointer contract.

### PR #4 — Normalize target specs

- **File:** `AetherOS/.cargo/aetheros-x86_64.json`
- **File:** `AetherOS/x86_64-unknown-none.json`
- **File:** `AetherOS/kernel/.cargo/aetheros-x86_64.json`
- **Summary:** Keep one canonical target JSON and update scripts/aliases to use it.
- **Reason:** Three copies invite drift and make linker/build fixes easy to miss.
- **Pitfalls:** Changing target filename changes Cargo target output directories; update QEMU scripts accordingly.

### PR #5 — Extract context switch assembly and call it from scheduler

- **File:** `AetherOS/kernel/src/arch/x86_64/context_switch.s`
- **File:** `AetherOS/kernel/src/task/context_switch.rs`
- **File:** `AetherOS/kernel/src/task/scheduler.rs`
- **Summary:** Move inline assembly to a standalone x86_64 assembly file or include it deterministically, then make scheduler dispatch call the low-level switch.
- **Reason:** Current scheduling does not actually transfer CPU context.
- **Pitfalls:** Do not hold scheduler locks across the context switch. Validate `TaskContext` layout offsets with compile-time assertions.

### PR #6 — Add a minimal address-space manager

- **File:** `AetherOS/kernel/src/memory/address_space.rs`
- **File:** `AetherOS/kernel/src/memory/mod.rs`
- **File:** `AetherOS/kernel/src/arch/x86_64/paging.rs`
- **File:** `AetherOS/kernel/src/task/tcb.rs`
- **Summary:** Create per-V-Node PML4s, map kernel global ranges, map user stack/segments, and track owned frames.
- **Reason:** V-Nodes currently use the kernel PML4 and are not isolated.
- **Pitfalls:** Accidentally mapping user pages without `USER_ACCESSIBLE`, or accidentally mapping kernel pages user-accessible, breaks isolation/security.

### PR #7 — Implement ELF `PT_LOAD` segment loading

- **File:** `AetherOS/kernel/src/elf.rs`
- **File:** `AetherOS/kernel/src/vnode_loader.rs`
- **Summary:** Parse ELF64 program headers, validate ranges, allocate/map user pages, copy file bytes, and zero BSS.
- **Reason:** Entry-point parsing alone cannot load a user binary.
- **Pitfalls:** Must reject non-canonical, kernel-range, overlapping, or integer-overflowing segment ranges.

### PR #8 — Complete timer scheduling dispatch milestone

- **File:** `AetherOS/kernel/src/timer.rs`
- **File:** `AetherOS/kernel/src/interrupts/timer.rs`
- **File:** `AetherOS/kernel/src/task/scheduler.rs`
- **Summary:** Keep deferred reschedule first, then add trap-frame save/restore on IRQ exit once context switching is safe.
- **Reason:** Timer accounting exists, but preemption is not real until CPU context is saved and restored.
- **Pitfalls:** Scheduler locks inside IRQ handlers can deadlock; IRQ code must only set flags or use IRQ-safe structures.

### PR #9 — Deduplicate PIC/IRQ initialization

- **File:** `AetherOS/kernel/src/lib.rs`
- **File:** `AetherOS/kernel/src/irq.rs`
- **File:** `AetherOS/kernel/src/interrupts/mod.rs`
- **File:** `AetherOS/kernel/src/arch/x86_64/irq.rs`
- **Summary:** Choose one PIC initialization path and make the other delegate or remove duplicate setup.
- **Reason:** Two independent PIC initialization flows make IRQ behavior hard to reason about.
- **Pitfalls:** Do not unmask IRQ lines before their IDT vectors are installed and loaded.

### PR #10 — Finish keyboard IRQ payload delivery

- **File:** `AetherOS/kernel/src/interrupts/keyboard.rs`
- **File:** `AetherOS/common/src/ipc/keyboard_ipc.rs`
- **Summary:** Serialize `KeyEvent` payloads with postcard or a fixed ABI payload and route to the registered keyboard V-Node.
- **Reason:** Current keyboard handler sends an empty payload.
- **Pitfalls:** Avoid allocating inside IRQ if allocator/locks are not IRQ-safe; a fixed-size ring buffer may be safer than `Vec` allocation.

### PR #11 — Bind a real network device to the stack

- **File:** `AetherOS/kernel/src/drivers/net.rs`
- **File:** `AetherOS/kernel/src/network.rs`
- **File:** `AetherOS/kernel/src/device.rs`
- **Summary:** Pick virtio-net or e1000, implement probe/init/read/write, register it with the device manager, and bind it to `NetworkStack`.
- **Reason:** Network logic exists but is not connected to real hardware.
- **Pitfalls:** DMA buffers must be physically contiguous or correctly described; interrupt ACK and descriptor ownership must be exact.

### PR #12 — Add persistent snapshot storage

- **File:** `AetherOS/kernel/src/snapshot_engine.rs`
- **File:** `AetherOS/kernel/src/aetherfs.rs`
- **Summary:** Implement `SnapshotStorage` over AetherFS/content-addressed storage and a persistent latest-snapshot pointer.
- **Reason:** In-memory snapshots cannot survive reboot.
- **Pitfalls:** Snapshot writes need atomicity; partial writes must not corrupt the latest pointer.

## Bootability roadmap

### Step 1 — Fix build system

- **Difficulty:** Medium
- **Required files:** `AetherOS/.cargo/config.toml`, target JSON, `AetherOS/scripts/build_kernel_image.sh`
- **Required code changes:** Add `-Zbuild-std` to build alias/script, pass linker script, normalize target output paths.
- **Exit criteria:** `cargo build-kernel` builds the bare-metal kernel ELF successfully from a clean checkout.

### Step 2 — Add or wire linker script

- **Difficulty:** Low/Medium
- **Required files:** `AetherOS/kernel/linker.ld`, target JSON or build script
- **Required code changes:** Ensure `rust-lld` receives `-Tkernel/linker.ld`; validate section order with objdump.
- **Exit criteria:** `.text._start`, `.rodata`, `.data`, `.bss`, stack symbols, and page-aligned sections match the linker script.

### Step 3 — Add bootloader configuration/image creation

- **Difficulty:** High
- **Required files:** `AetherOS/scripts/build_kernel_image.sh`, `AetherOS/scripts/run_qemu.sh`, maybe `AetherOS/bootimage.toml` only if bootimage is adopted
- **Required code changes:** Produce either a UEFI ESP/disk image or bootloader-wrapped BIOS image that hands `BootInfo` to `_start`.
- **Exit criteria:** QEMU reaches serial output from `kernel_entry` using the intended boot protocol.

### Step 4 — Implement context switch assembly integration

- **Difficulty:** High
- **Required files:** `AetherOS/kernel/src/task/context_switch.rs`, optional `AetherOS/kernel/src/arch/x86_64/context_switch.s`, `AetherOS/kernel/src/task/scheduler.rs`
- **Required code changes:** Call low-level switch from scheduler without holding global locks; add CR3 switch and first-run path.
- **Exit criteria:** Two kernel tasks can cooperatively yield and resume with distinct stacks/registers.

### Step 5 — Wire timer IRQ to scheduling safely

- **Difficulty:** Medium/High
- **Required files:** `AetherOS/kernel/src/timer.rs`, `AetherOS/kernel/src/interrupts/timer.rs`, `AetherOS/kernel/src/task/scheduler.rs`
- **Required code changes:** Preserve deferred reschedule first; later add IRQ-exit context capture for real preemption.
- **Exit criteria:** Timer quantum causes task rotation without waiting for manual yields.

### Step 6 — Implement V-Node address-space isolation

- **Difficulty:** High
- **Required files:** `AetherOS/kernel/src/memory/address_space.rs`, `AetherOS/kernel/src/arch/x86_64/paging.rs`, `AetherOS/kernel/src/task/tcb.rs`, `AetherOS/kernel/src/vnode_loader.rs`
- **Required code changes:** Per-V-Node page tables, user stack mapping, CR3 root tracking, page ownership cleanup.
- **Exit criteria:** User/V-Node pages are mapped user-accessible; kernel pages remain supervisor-only; page faults terminate only the offending V-Node.

### Step 7 — Complete ELF loader

- **Difficulty:** High
- **Required files:** `AetherOS/kernel/src/elf.rs`, `AetherOS/kernel/src/vnode_loader.rs`, address-space manager file
- **Required code changes:** Parse and map `PT_LOAD` program headers; copy bytes; zero BSS; enforce flags and ranges.
- **Exit criteria:** A minimal V-Node ELF can be loaded into its own address space and entered at its ELF entry point.

### Step 8 — Complete device and snapshot support after boot

- **Difficulty:** Medium/High
- **Required files:** `AetherOS/kernel/src/interrupts/keyboard.rs`, `AetherOS/kernel/src/drivers/net.rs`, `AetherOS/kernel/src/network.rs`, `AetherOS/kernel/src/snapshot_engine.rs`, `AetherOS/kernel/src/aetherfs.rs`
- **Required code changes:** Keyboard payload ABI, real NIC driver path, RX/TX integration, persistent snapshot storage.
- **Exit criteria:** Keyboard and network V-Nodes receive real events; snapshots persist across reboot.

## Optional minimal code snippets

These snippets are intentionally skeletal and should be adapted to the existing paging and allocator APIs.

### Minimal ELF program-header type

```rust
#[derive(Debug, Clone, Copy)]
pub struct ElfProgramHeader {
    pub typ: u32,
    pub flags: u32,
    pub offset: u64,
    pub vaddr: u64,
    pub filesz: u64,
    pub memsz: u64,
    pub align: u64,
}
```

### Minimal address-space API shape

```rust
pub struct AddressSpace {
    pub root_pml4_phys: u64,
    pub owned_pages: alloc::vec::Vec<x86_64::VirtAddr>,
}

impl AddressSpace {
    pub fn new_user() -> Result<Self, &'static str>;
    pub fn map_user_segment(&mut self, vaddr: u64, bytes: &[u8], memsz: u64, flags: u32) -> Result<(), &'static str>;
}
```

### Minimal snapshot backend direction

```rust
pub struct AetherFsSnapshotStorage;

impl SnapshotStorage for AetherFsSnapshotStorage {
    fn load_latest(&self) -> Option<Vec<u8>> { /* read latest pointer, then blob */ None }
    fn load_by_id(&self, id: u64) -> Option<Vec<u8>> { /* read indexed blob */ None }
    fn store(&mut self, id: u64, data: &[u8]) -> Result<(), SnapshotError> { /* atomic blob + pointer */ Ok(()) }
}
```
