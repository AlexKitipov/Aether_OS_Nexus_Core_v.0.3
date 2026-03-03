# WebView Renderer V-Node

## Overview

The WebView Renderer V-Node is responsible for parsing, styling, laying out, and rendering web content (HTML, CSS, JavaScript) into a pixel buffer. It acts as a client to the Display Compositor V-Node, sending rendered frames for display.

## Core Responsibilities

*   **HTML Parsing**: Converts raw HTML into a Document Object Model (DOM) tree.
*   **CSS Styling**: Applies CSS rules to the DOM elements to compute their final visual properties.
*   **Layout Management**: Calculates the position and size of each element on the screen.
*   **Rendering**: Draws the styled and laid-out elements onto a pixel buffer (framebuffer).
*   **IPC Communication**: Communicates with the `Display Compositor` to request new windows, send rendered frames, and receive UI events (mouse, keyboard).
*   **Network Interaction**: Uses `socket-api` and `dns-resolver` to fetch web resources (e.g., images, scripts, stylesheets).
*   **Resource Caching**: Manages a local cache of web resources to improve performance.

## Capabilities and Dependencies

To perform its functions, the WebView V-Node requires specific capabilities:

*   `CAP_IPC_CONNECT: "svc://ui-compositor"`: To send rendering instructions and receive events from the compositor.
*   `CAP_IPC_CONNECT: "svc://dns-resolver"`: To resolve hostnames encountered in URLs.
*   `CAP_IPC_CONNECT: "svc://socket-api"`: To make network requests (HTTP, WebSockets) for fetching web content.
*   `CAP_IPC_CONNECT: "svc://vfs"`: To load local web assets, manage cache, and handle downloads.
*   `CAP_LOG_WRITE`: For debugging and logging web content errors or network activity.
*   `CAP_TIME_READ`: For JavaScript timers and network timeouts.
*   `CAP_MEM_SHARE`: Crucial for zero-copy transfer of rendered pixel data to the compositor.

## Operational Flow (High-Level)

1.  **Initialization**: Requests a new window from the `Display Compositor`.
2.  **Content Loading**: Fetches HTML, CSS, and other resources via network/VFS.
3.  **Processing**: Parses HTML, processes CSS, computes layout.
4.  **Rendering Loop**: Draws the content to an internal pixel buffer.
5.  **Display**: Sends the pixel buffer (or updates) to the `Display Compositor` via IPC.
6.  **Event Handling**: Receives UI events from the `Display Compositor` and dispatches them to web content (e.g., JavaScript).
