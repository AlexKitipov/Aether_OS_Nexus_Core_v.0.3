# Display Compositor V-Node

## Overview

The Display Compositor V-Node is a central component of the AetherOS UI subsystem, acting as the primary interface between UI applications (like the `WebView Renderer`) and the underlying GPU driver. Its main responsibility is to manage window surfaces, composite them into a single visual output, and handle user input events.

## Core Responsibilities

*   **Window Management**: Creates, tracks, and destroys window surfaces requested by client V-Nodes.
*   **Composition**: Receives pixel data (rendered frames) from various UI V-Nodes and composites them into a unified framebuffer, respecting Z-order and damage regions.
*   **GPU Interaction**: Interacts with the `VirtIO-GPU Driver` (or similar low-level graphics driver) to push the composed framebuffer to the display hardware.
*   **Input Handling**: Receives raw input events (mouse, keyboard) from the `Nexus Input Bridge V-Node`, performs hit-testing to identify the target window, and routes these events to the appropriate client UI V-Node.
*   **Focus Management**: Determines which window has input focus and directs keyboard events accordingly.
*   **Zero-Copy Rendering**: Leverages shared memory and DMA capabilities for efficient, zero-copy transfer of pixel data from rendering V-Nodes to its internal buffers and then to the GPU driver.

## Capabilities and Dependencies

To perform its functions, the Display Compositor V-Node requires extensive capabilities:

*   `CAP_IPC_ACCEPT`: To accept `UiRequest` messages (like `CreateWindow`, `DrawToSurface`) from client UI V-Nodes.
*   `CAP_IPC_CONNECT: "svc://virtio-gpu-driver"`: To send framebuffer updates and receive display configuration.
*   `CAP_IPC_CONNECT: "svc://nexus-input-bridge"`: To receive raw keyboard and mouse events.
*   `CAP_LOG_WRITE`: For debugging, logging composition events, and input routing.
*   `CAP_TIME_READ`: For managing animations, event timestamps, and composition timing.
*   `CAP_MEM_SHARE`: Critical for sharing framebuffer memory with rendering V-Nodes and the GPU driver, enabling zero-copy data flow.

## Operational Flow (High-Level)

1.  **Initialization**: Establishes connections with the `VirtIO-GPU Driver` and `Nexus Input Bridge`.
2.  **Window Creation**: Receives `UiRequest::CreateWindow` from a client, allocates a window surface, and returns a `window_id`.
3.  **Rendering Loop**: 
    a.  Receives `UiRequest::DrawToSurface` messages with pixel data for specific windows.
    b.  Updates its internal representation of the window surfaces.
    c.  Composites all visible window surfaces into a single scene.
    d.  Sends the final composed image (or changed regions) to the `VirtIO-GPU Driver` for display.
4.  **Input Loop**: 
    a.  Receives `InputEvent` messages (raw mouse/keyboard data) from `Nexus Input Bridge`.
    b.  Determines which window (if any) is under the mouse cursor or has focus.
    c.  Translates raw input into high-level `MouseEvent` or `KeyEvent` messages.
    d.  Sends these events via IPC to the appropriate client V-Node (e.g., `WebView Renderer`).

This architecture ensures that the critical task of display composition and input routing is isolated and highly privileged, forming the visual backbone of AetherOS.
