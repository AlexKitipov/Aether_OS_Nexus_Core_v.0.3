# Net-Bridge V-Node

## Overview

The `net-bridge` V-Node is a low-level system driver responsible for bridging the AetherOS network stack (`aethernet-service`) with the physical (or virtual) network interface controller (NIC) hardware. It abstracts away the complexities of direct hardware interaction and provides a standardized IPC interface for the network stack.

## Core Responsibilities

*   **Hardware Abstraction**: Manages direct communication with the NIC hardware, typically through memory-mapped I/O (MMIO) and DMA (Direct Memory Access).
*   **Packet Reception (Rx)**: Polls the NIC for incoming network packets, copies them into DMA-allocated buffers, and forwards their handles and lengths to the `aethernet-service` V-Node via IPC.
*   **Packet Transmission (Tx)**: Receives DMA buffer handles and lengths from `aethernet-service` via IPC, and instructs the NIC to transmit these packets.
*   **Interrupt Handling**: Registers with the kernel for NIC-specific interrupts (e.g., packet received, transmission complete) and acknowledges them. It then signals the `aethernet-service` about relevant events.
*   **Zero-Copy Networking**: Utilizes kernel-managed DMA buffers and memory sharing capabilities (`CAP_MEM_SHARE`) to enable efficient, zero-copy data transfer between the NIC, `net-bridge`, and `aethernet-service`.

## Capabilities and Dependencies

To perform its functions, the `net-bridge` V-Node requires highly privileged capabilities:

*   `NetworkAccess`: A broad capability that encompasses various network-related operations.
*   `CAP_HARDWARE_IO_PCI: "0x00:0x1F:0x0"`: Direct access to a specific PCI device (e.g., a VirtIO-Net NIC). This allows configuration of device registers and queues.
*   `CAP_IRQ_REGISTER: 11`: The ability to register an interrupt handler for a specific IRQ line (e.g., IRQ 11 for VirtIO-Net devices). The kernel routes these hardware interrupts to the V-Node via IPC.
*   `CAP_DMA_ALLOC`: Permission to allocate physically contiguous, DMA-capable memory buffers from the kernel. These buffers are used for packet exchange with the NIC.
*   `CAP_MEM_SHARE`: Crucial for transferring ownership or sharing memory regions (DMA buffers) with other V-Nodes (like `aethernet-service`) without physically copying data.
*   `CAP_SYSCALL_RAW`: (Potentially needed) For highly specialized direct hardware access or low-level MMIO if not covered by other capabilities.
*   `CAP_LOG_WRITE`: For logging driver events, errors, and debugging network issues.

## Operational Flow (High-Level)

1.  **Initialization**: 
    *   Registers its IRQ handler with the kernel. 
    *   Allocates an initial pool of DMA buffers for packet reception.
    *   Establishes IPC channels with `aethernet-service`.
2.  **Packet Reception**: 
    *   Receives IRQ events from the kernel, signaling incoming packets. 
    *   Uses a kernel syscall (`SYS_NET_RX_POLL`) to retrieve packets from the NIC into a DMA buffer. 
    *   Sends a `NetPacketMsg::RxPacket` message (containing the DMA handle and length) to `aethernet-service` via IPC. Ownership of the buffer is conceptually transferred.
    *   Acknowledges the IRQ to the kernel.
3.  **Packet Transmission**: 
    *   Receives `NetPacketMsg::TxPacket` messages (containing a DMA handle and length) from `aethernet-service` via IPC. 
    *   Uses a kernel syscall (`SYS_NET_TX`) to instruct the NIC to transmit the data from the provided DMA buffer. 
    *   Frees the DMA buffer after transmission (or returns it to a pool).
    *   Sends a `NetPacketMsg::TxPacketAck` back to `aethernet-service`.
4.  **Event Loop**: Continuously polls its IPC channels for incoming messages from the kernel (IRQs) and `aethernet-service` (Tx requests), processing them efficiently.

## Example `vnode.yml` Configuration

```yaml
# vnode/net-bridge/vnode.yml
vnode:
  name: "net-bridge"
  version: "0.1.0"
  maintainer: "aetheros-core-team@aetheros.org"
  mode: strict # System drivers should always be strict for security

runtime:
  entrypoint: "bin/net-bridge.vnode"
  required_mem_mb: 16 # Minimal memory for a driver V-Node
  max_cpu_share: 0.05 # Limited CPU share as it's primarily I/O bound

capabilities:
  - NetworkAccess # General capability for network operations
  - CAP_HARDWARE_IO_PCI: "0x00:0x1F:0x0" # Access to specific PCI device (e.g., VirtIO-Net NIC)
  - CAP_IRQ_REGISTER: 11 # Register IRQ 11 for VirtIO-Net
  - CAP_DMA_ALLOC # Allocate DMA-compatible memory pages for network buffers
  - CAP_MEM_SHARE # For zero-copy data transfer with `svc://aethernet`
  - CAP_SYSCALL_RAW # Potentially for direct hardware register access (MMIO) if needed

storage:
  mounts: [] # As a low-level driver, it typically doesn't need persistent storage

observability:
  metrics: ["packets_rx", "packets_tx", "irq_latency_ns"]
```
