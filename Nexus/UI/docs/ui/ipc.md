# UI IPC Protocol

## Overview

The UI IPC (Inter-Process Communication) Protocol defines the standardized messages and structures used for communication between various UI-related V-Nodes within AetherOS. This protocol ensures loose coupling and clear contracts between components like the `WebView Renderer`, `Display Compositor`, and client applications.

## Key Principles

*   **Message-Based**: All communication is conducted via serialized messages (using `postcard` for efficiency) over dedicated IPC channels.
*   **Capability-Secured**: Every message exchange is subject to the kernel's capability-based security model, ensuring that V-Nodes only communicate and perform actions for which they have explicit permissions.
*   **Asynchronous**: While `send_and_recv` patterns can simulate synchronous calls, the underlying IPC mechanism is asynchronous, allowing V-Nodes to remain responsive.
*   **Zero-Copy Potential**: For large data transfers, such as pixel buffers, the protocol is designed to leverage shared memory and DMA handles, enabling zero-copy data exchange where possible.

## Core Messages (Defined in `common/src/ipc/ui_protocol.rs`)

The primary enums defining the UI IPC protocol are `UiRequest` and `UiResponse`:

### `UiRequest`

Messages sent *to* UI services (e.g., `Display Compositor`) from client V-Nodes:

*   `CreateWindow { title: String, width: u32, height: u32 }`:
    *   **Purpose**: Requests the creation of a new window surface on the display.
    *   **Sender**: Any V-Node needing a graphical output (e.g., `WebView Renderer`, `AetherTerminal`).
    *   **Recipient**: `svc://ui-compositor`.

*   `DrawToSurface { window_id: u32, x: u32, y: u32, width: u32, height: u32, pixels: Vec<u8> }`:
    *   **Purpose**: Sends pixel data to be drawn onto a specific window surface.
    *   **Sender**: V-Nodes that render graphical content.
    *   **Recipient**: `svc://ui-compositor`.

*   `MouseEvent { window_id: u32, x: u32, y: u32, button: u8, event_type: MouseEventType }`:
    *   **Purpose**: In a reverse flow, this could be sent *from* the Compositor to a client V-Node to notify about mouse interactions. (Currently defined as a request to simplify initial implementation).
    *   **Sender**: `svc://ui-compositor` (conceptually), or a mock input driver.
    *   **Recipient**: Client V-Node (e.g., `WebView Renderer`) which owns the window.

*   `KeyEvent { window_id: u32, keycode: u16, event_type: KeyEventType }`:
    *   **Purpose**: Similar to `MouseEvent`, notifies about keyboard interactions.
    *   **Sender**: `svc://ui-compositor` (conceptually).
    *   **Recipient**: Client V-Node.

*   `CloseWindow { window_id: u32 }`:
    *   **Purpose**: Requests the closing and destruction of a window surface.
    *   **Sender**: Client V-Nodes.
    *   **Recipient**: `svc://ui-compositor`.

*   `GetWindows`:
    *   **Purpose**: Queries the compositor for a list of all active windows.
    *   **Sender**: Diagnostic tools, shell, or other management V-Nodes.
    *   **Recipient**: `svc://ui-compositor`.

### `UiResponse`

Messages sent *from* UI services (e.g., `Display Compositor`) back to client V-Nodes:

*   `Success { window_id: Option<u32> }`:
    *   **Purpose**: Indicates that a requested operation completed successfully. May return a new `window_id` if one was created.

*   `Windows(Vec<WindowInfo>)`:
    *   **Purpose**: Returns a list of `WindowInfo` structures, providing details about currently active windows.

*   `Error { message: String }`:
    *   **Purpose**: Signals that an operation failed, with a descriptive error message.

## Flow Example: WebView Rendering a Page

1.  **WebView** sends `UiRequest::CreateWindow` to `Display Compositor` (e.g., via channel ID 12).
2.  **Display Compositor** creates internal window state, returns `UiResponse::Success { window_id: Some(id) }`.
3.  **WebView** loads HTML/CSS, renders to a pixel buffer.
4.  **WebView** sends `UiRequest::DrawToSurface { window_id: id, ... }` with pixel data to `Display Compositor`.
5.  **Display Compositor** composites the pixels onto the virtual framebuffer and responds `UiResponse::Success` (or `Error`).
6.  If a user interacts with the window (e.g., mouse click), **Display Compositor** sends `UiRequest::MouseEvent` (conceptually acting as a notification) to the **WebView** V-Node to handle the event.

This modular approach ensures that UI components are isolated, robust, and debuggable, aligning with the Nexus Hybrid architecture.
