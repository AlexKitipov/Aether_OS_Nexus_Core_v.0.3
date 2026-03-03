
import asyncio
import websockets
import json

# Assuming colab_ui_test.py is in the same directory and has `renderer` and `process_ipc_message` defined.
# For a true isolated module, you'd import specific functions.
from colab_ui_test import process_ipc_message, renderer

# In-memory channel for simulated IPC from kernel/V-nodes to the Colab UI
# This simulates the IPC channel where the kernel/compositor would send messages
# which are then picked up by the WebSocket server.
ipc_message_queue = asyncio.Queue()

async def handle_websocket(websocket, path):
    print(f"WebSocket connection established: {path}")
    try:
        async for message in websocket:
            print(f"Received message from client: {message[:100]}...") # Log first 100 chars
            try:
                # Attempt to parse as JSON. This is how V-nodes would communicate.
                parsed_message = json.loads(message)
                
                # Simulate kernel IPC by processing the message via the renderer
                response = await process_ipc_message(parsed_message)
                
                # Send response back to the client
                await websocket.send(json.dumps(response))
                print(f"Sent response to client: {json.dumps(response)[:100]}...")

            except json.JSONDecodeError:
                print("Received non-JSON message.")
                await websocket.send(json.dumps({"type": "Error", "payload": {"message": "Invalid JSON"}}))
            except Exception as e:
                print(f"Error processing message: {e}")
                await websocket.send(json.dumps({"type": "Error", "payload": {"message": str(e)}}))

    except websockets.exceptions.ConnectionClosedOK:
        print("WebSocket connection closed normally.")
    except websockets.exceptions.ConnectionClosedError as e:
        print(f"WebSocket connection closed with error: {e}")
    except Exception as e:
        print(f"An unexpected error occurred in WebSocket handler: {e}")
    finally:
        print(f"WebSocket connection terminated: {path}")


async def start_websocket_server(port=8765):
    # Ensure the renderer is initialized and its display method doesn't block
    renderer.start_ui_renderer() 

    server = await websockets.serve(handle_websocket, "0.0.0.0", port)
    print(f"WebSocket server started on ws://0.0.0.0:{port}")
    await server.wait_closed()

# Example usage in a Colab cell:
# from google.colab import output
# output.serve_kernel_port_as_iframe(8765)
# await start_websocket_server()

# This function needs to be called in an async context
# To run this directly in Colab (blocking current cell):
# await start_websocket_server()

# Or to run in background:
# import asyncio
# asyncio.create_task(start_websocket_server())
