# AetherOS UI Subsystem

This directory contains the core components and documentation for the AetherOS User Interface subsystem. The UI is designed as a microkernel-style architecture, with isolated V-Nodes handling different aspects of the graphical environment, from web rendering to display composition.

## Key Components:

*   **WebView Renderer V-Node**: Responsible for parsing HTML, applying CSS, performing layout, and rendering web content into a pixel buffer.
*   **Display Compositor V-Node**: Manages multiple window surfaces, receives rendered frames from client V-Nodes, and composites them onto the virtual framebuffer.
*   **UI IPC Protocol**: Defines the communication interface between UI V-Nodes and client applications.
*   **Layout Engine**: Handles the calculation of element positions and sizes based on parsed HTML and CSS.
*   **Colab Testing Tools**: Python scripts for simulating the UI Compositor and displaying rendered output directly within a Google Colab environment.

## Getting Started:

Refer to the `docs/` directory for detailed information on each component and how to interact with the UI subsystem.
