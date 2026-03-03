# Colab-Friendly UI Testing Setup

## Overview

To facilitate development and demonstration within the Google Colab environment, a specialized testing setup has been designed. This setup allows for the simulation of the AetherOS UI Compositor and the visualization of rendered output directly within a Colab notebook, bypassing the need for a full QEMU-based graphical environment for basic UI component testing.

## Components

1.  **`colab_ui_test.py` (Mock UI Renderer)**:
    *   **Purpose**: This Python module provides a mock implementation of a UI renderer, including `MockWindow` objects that simulate window surfaces and framebuffers. It processes UI IPC requests (like `CreateWindow`, `DrawToSurface`) and renders them using `PIL` (Pillow) and `numpy`.
    *   **Functionality**: It maintains a collection of `MockWindow` instances, each with its own framebuffer. When `DrawToSurface` requests come in, it updates the corresponding mock framebuffer. It then uses `IPython.display` to render the most recent or active window's content as a PNG image in the Colab output.

2.  **`colab_websocket_bridge.py` (WebSocket IPC Bridge)**:
    *   **Purpose**: This module sets up a local WebSocket server within the Colab environment. It acts as a bridge, listening for incoming JSON messages (simulating serialized IPC from AetherOS V-Nodes) and forwarding them to the `colab_ui_test.py` renderer for processing.
    *   **Functionality**: It uses the `websockets` library to create a server. When a client (e.g., a separately run Python script or even a conceptual V-Node simulator) connects and sends a JSON message representing a `UiRequest`, the bridge deserializes it, passes it to the `process_ipc_message` function from `colab_ui_test.py`, and sends back the `UiResponse`.

## How to Use (Conceptual Steps within Colab)

1.  **Start the WebSocket Server**:
    ```python
    from google.colab import output
    output.serve_kernel_port_as_iframe(8765) # Makes the local server accessible via iframe
    import asyncio
    from tools.colab_websocket_bridge import start_websocket_server
    asyncio.create_task(start_websocket_server(port=8765))
    ```
    This will start the WebSocket server in the background and display an empty iframe, ready to receive UI commands.

2.  **Simulate V-Node Communication**:
    You would then write a Python script (or conceptually, a compiled AetherOS V-Node communicating via an external client) that connects to `ws://localhost:8765` (via the iframe) and sends `UiRequest` messages as JSON.

    ```python
    # Example: Python client connecting to the WebSocket bridge
    import asyncio
    import websockets
    import json
    import base64
    import numpy as np

    async def send_ui_command():
        uri = "ws://localhost:8765" # Colab redirects this port via iframe
        async with websockets.connect(uri) as websocket:
            print("Client: Connected to WebSocket.")

            # Request to create a window
            create_req = {
                "type": "CreateWindow",
                "payload": {
                    "title": "My Web View",
                    "width": 400,
                    "height": 300
                }
            }
            await websocket.send(json.dumps(create_req))
            response = await websocket.recv()
            print(f"Client: Received response: {response}")
            window_id = json.loads(response)["payload"]["window_id"]

            # Prepare some dummy pixel data (e.g., a red square)
            pixels = np.zeros((300, 400, 4), dtype=np.uint8)
            pixels[:, :, 0] = 255 # Red channel
            pixels[:, :, 3] = 255 # Alpha channel
            pixels_b64 = base64.b64encode(pixels.tobytes()).decode('utf-8')

            # Request to draw to surface
            draw_req = {
                "type": "DrawToSurface",
                "payload": {
                    "window_id": window_id,
                    "x": 0,
                    "y": 0,
                    "width": 400,
                    "height": 300,
                    "pixels": pixels_b64
                }
            }
            await websocket.send(json.dumps(draw_req))
            response = await websocket.recv()
            print(f"Client: Received response: {response}")

            await asyncio.sleep(2) # Keep connection open to see the render

    # asyncio.run(send_ui_command())
    ```
    (Note: Running `asyncio.run` directly in Colab might interfere with the event loop. Use `asyncio.create_task` or `await` within an already running event loop.)

## Benefits

*   **Rapid Iteration**: Quickly test UI logic and rendering without recompiling and running the entire OS in QEMU.
*   **Visual Feedback**: See immediate graphical output within the notebook.
*   **Debugging**: Easier to inspect intermediate states and log messages within a Python environment.

This setup provides a powerful tool for developing and validating the AetherOS UI subsystem in an interactive and efficient manner.