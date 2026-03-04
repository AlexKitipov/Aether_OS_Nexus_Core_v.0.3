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

## 🛠️ Build & Run Guide (Conceptual)

This guide outlines the conceptual steps to build and run AetherOS Nexus Core in a simulated environment (QEMU).

### Prerequisites

*   **Rust Nightly**: Ensure you have a recent nightly Rust toolchain installed.
*   **`rust-src` component**: `rustup component add rust-src --toolchain nightly`
*   **`llvm-tools-preview` component**: `rustup component add llvm-tools-preview`
*   **`bootimage` cargo subcommand**: `cargo install bootimage`
*   **QEMU**: Version 5.2 or newer, for `x86_64` architecture (`qemu-system-x86_64`).

### Building AetherOS Nexus Core

1.  **Clone the repository**:
    ```bash
    git clone https://github.com/aetheros/nexus-core.git # (Conceptual URL)
    cd nexus-core
    ```
2.  **Install `bootimage`**:
    ```bash
    cargo install bootimage --version <version>
    ```
3.  **Compile V-Node applications**:
    Each V-Node (`vnode/*`) is compiled as a separate `no_std` ELF binary.
    ```bash
    # Example for registry V-Node
    cargo build -p vnode-registry --target x86_64-unknown-none --release
    # Repeat for other V-Nodes (net-bridge, net-stack, etc.)
    ```
4.  **Create `initrd` (Initial RAM Disk)**:
    This step bundles your compiled V-Node binaries and their manifests into a single image that the kernel will load at boot.
    ```bash
    # Conceptual: Use a script to package V-Nodes into an initrd image.
    # For v0.1, AetherFS is very basic and might just expect a single V-Node binary for testing.
    ```
5.  **Build the Kernel**:
    The `bootimage` tool compiles the `kernel` crate and embeds your `initrd` (if configured) into a bootable `ELF` kernel image.
    ```bash
    cd kernel
    cargo bootimage --release
    # This will generate a bootable image at target/x86_64-unknown-none/release/bootimage-aetheros-kernel.bin
    ```

### 🚀 Running in QEMU

To see AetherOS Nexus Core in action:

```bash
qemu-system-x86_64 \
  -machine q35 \
  -m 2G \
  -serial stdio \
  -drive format=raw,file=kernel/target/x86_64-unknown-none/release/bootimage-aetheros-kernel.bin \
  # Add -initrd <path_to_your_initrd> if you have one prepared
  # For network simulation (if enabled and configured):
  -netdev user,id=net0,hostfwd=tcp::8080-:80 \
  -device virtio-net-pci,netdev=net0,mac=02:00:00:00:00:01
```

All kernel and V-Node logs will be streamed to your console via the `-serial stdio` option.

**Join the Aether. Build the Nexus.**
