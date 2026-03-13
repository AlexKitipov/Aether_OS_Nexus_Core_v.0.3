# 🌌 AetherOS Alpha — The Nexus Architecture

_Join the Aether. Build the Nexus._

## 🚀 Project Vision & Mission

AetherOS is not just another operating system; it's a **Nexus Hybrid** – a new class of operating system designed from the ground up to redefine security, performance, and transparency in computing. Our mission is to build a platform that is robust, user-centric, and resilient in an increasingly complex digital world, empowering developers and users with unprecedented control and insight.

Traditional operating systems are prisoners of their history:

*   **Windows** is a monolithic labyrinth of legacy code, constantly battling security vulnerabilities and resource inefficiency.
*   **Linux** is powerful but fragmented, often requiring deep technical expertise for optimal configuration.
*   **macOS** offers a polished experience but confines users to a closed ecosystem, limiting freedom and transparency.

None of them are built for a world where drivers are sandboxed by default, inter-process communication (IPC) is visually inspectable, and applications are immutable, cryptographically verifiable entities. AetherOS aims to be that paradigm shift.

## 🧬 Core Architectural Pillars (Alpha Complete)

AetherOS is founded on revolutionary principles that leverage modern systems programming and cryptographic guarantees, demonstrated across several stages:

1.  **Memory Safety by Default**: The entire Nexus Core is written in **Rust**, eliminating 70% of classic kernel vulnerabilities.
2.  **Nexus Hybrid Microkernel**: A minimal, capability-secured microkernel manages only memory, CPU scheduling, and IPC.
3.  **Capability-Based Security**: No `root` user. Every V-Node possesses only explicitly granted rights.
4.  **Zero-Copy IPC**: IPC is designed for lightning speed using shared memory with transfer-of-ownership semantics.
5.  **Zero-Trust Runtime**: No component is inherently trusted; every operation is validated.
6.  **Immutable Infrastructure (V-Nodes)**: Applications as cryptographically signed, content-addressed, immutable bundles.
7.  **Zero-Copy Networking**: Data moves from NIC to application without CPU-intensive copying.
8.  **Visual Observability**: Real-time, interactive visualization of IPC flows, V-Node states, and resource usage.
9.  **Aether Driver Intelligence (ADI)**: AI-assisted system to translate existing drivers into safe, sandboxed V-Nodes.
10. **Decentralized Trust Model**: Cryptographic trust with Merkle Trees and Content-Addressable Storage.
11. **Resource Quotas & Admission Control**: Every V-Node declares its resource needs, enforced by the Nexus Core.

## 📁 Project Structure

```text
aetheros/
├─ Cargo.toml                  # Workspace root
├─ kernel/                     # The Nexus Core (operating system kernel)
│  ├─ Cargo.toml
│  ├─ src/
│  │  ├─ arch/x86_64/         # x86_64 architecture-specific code (boot, GDT, IDT, paging, DMA, IRQ)
│  │  ├─ drivers/             # Device drivers (e.g., serial)
│  │  ├─ memory/              # Memory management (frame allocator, page allocator)
│  │  ├─ task/                # Task management (TCB, scheduler)
│  │  ├─ ipc/                 # Inter-Process Communication (mailbox)
│  │  ├─ console.rs           # Kernel console output
│  │  ├─ timer.rs             # Kernel timer
│  │  ├─ caps.rs              # Capability definitions
│  │  ├─ syscall.rs           # Syscall dispatcher
│  │  ├─ lib.rs               # Kernel library entry point, module declarations
│  │  ├─ main.rs              # Kernel main entry point (_start, panic_handler)
│  │  ├─ aetherfs.rs          # AetherFS conceptual implementation
│  │  ├─ elf.rs               # ELF loader conceptual implementation
│  │  └─ vnode_loader.rs      # V-Node loader conceptual implementation
│  └─ linker.ld
├─ common/                     # Common utilities and IPC message definitions for kernel and V-Nodes
│  ├─ Cargo.toml
│  ├─ src/
│  │  ├─ ipc/                  # IPC messaging definitions
│  │  ├─ syscall.rs            # User-space syscall wrappers
│  │  └─ lib.rs                # Common library entry point
├─ vnode/                      # Example V-Node applications
│  ├─ dns-resolver/             # DNS Resolver V-Node
│  ├─ file-manager/             # File Manager V-Node
│  ├─ init-service/             # Init Service V-Node
│  ├─ mail-service/             # Mail Service V-Node
│  ├─ model-runtime/            # Model Runtime V-Node
│  ├─ net-bridge/               # Network Bridge Driver V-Node
│  ├─ net-stack/                # AetherNet Network Stack V-Node
│  ├─ registry/                 # Package Registry V-Node
│  ├─ shell/                    # Shell V-Node
│  ├─ socket-api/               # Socket API V-Node
│  └─ vfs/                      # Virtual File System V-Node
```

## 🛠️ Build & Run Guide (bootloader_api 0.11)

This project uses the modern `bootloader_api` flow. Legacy `bootloader` 0.10 / `bootimage` commands are not used.

### Prerequisites

- Rust nightly
- `rust-src` and `llvm-tools-preview` components for nightly
- QEMU (`qemu-system-x86_64`)

```bash
rustup toolchain install nightly
rustup component add rust-src --toolchain nightly
rustup component add llvm-tools-preview --toolchain nightly
```

### Build kernel

From `AetherOS/`:

```bash
cargo +nightly build \
  -Z json-target-spec \
  -Z build-std=core,alloc,compiler_builtins \
  -Z build-std-features=compiler-builtins-mem \
  -p aetheros-kernel \
  --manifest-path kernel/Cargo.toml \
  --target kernel/.cargo/aetheros-x86_64.json \
  --release
```

Or use the helper:

```bash
./scripts/build_kernel_image.sh
```

### Run in QEMU

```bash
qemu-system-x86_64 -kernel target/aetheros-x86_64/release/aetheros-kernel
```

### Workspace helper flow

```bash
./scripts/build_all.sh
./scripts/build_initrd.sh
./scripts/run_qemu.sh
```

## 🔧 Troubleshooting workspace build errors

If you are validating user-space V-Node services (such as `registry` and `init-service`), build those packages directly from the workspace root:

```bash
cd AetherOS
cargo build -p registry -p init-service
```

This avoids mixing kernel/nightly-only targets with host-side service validation and provides faster feedback for IPC/API-level changes.

**Join the Aether. Build the Nexus.**
