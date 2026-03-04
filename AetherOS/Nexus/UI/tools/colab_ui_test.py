
import asyncio
import base64
import json
import numpy as np
from PIL import Image
from io import BytesIO
from IPython.display import display, Image as IPImage, clear_output

# Mock representation of a window
class MockWindow:
    def __init__(self, window_id, title, width, height):
        self.id = window_id
        self.title = title
        self.width = width
        self.height = height
        self.framebuffer = np.zeros((height, width, 4), dtype=np.uint8) # RGBA

    def draw_to_surface(self, x, y, width, height, pixels):
        # Convert flattened RGBA byte array to numpy array
        pixels_np = np.array(pixels, dtype=np.uint8).reshape((height, width, 4))
        # Copy pixels to the window's framebuffer
        self.framebuffer[y:y+height, x:x+width] = pixels_np

    def get_image(self):
        return Image.fromarray(self.framebuffer, 'RGBA')


class ColabUIRenderer:
    def __init__(self):
        self.windows = {}
        self.next_window_id = 1
        print("Colab UI Renderer initialized.")

    async def handle_request(self, request):
        req_type = request.get("type")
        payload = request.get("payload", {})
        
        print(f"Renderer: Received request type: {req_type}")

        if req_type == "CreateWindow":
            window_id = self.next_window_id
            self.next_window_id += 1
            window = MockWindow(window_id, payload["title"], payload["width"], payload["height"])
            self.windows[window_id] = window
            print(f"Renderer: Created window {window_id} - '{window.title}' ({window.width}x{window.height})")
            return {"type": "Success", "payload": {"window_id": window_id}}

        elif req_type == "DrawToSurface":
            window_id = payload["window_id"]
            x, y, width, height = payload["x"], payload["y"], payload["width"], payload["height"]
            pixels_b64 = payload["pixels"]
            pixels = base64.b64decode(pixels_b64)

            if window_id in self.windows:
                self.windows[window_id].draw_to_surface(x, y, width, height, pixels)
                self.render_all_windows()
                return {"type": "Success", "payload": {"window_id": window_id}}
            else:
                print(f"Renderer: Error - Window {window_id} not found for DrawToSurface.")
                return {"type": "Error", "payload": {"message": f"Window {window_id} not found"}}

        elif req_type == "CloseWindow":
            window_id = payload["window_id"]
            if window_id in self.windows:
                del self.windows[window_id]
                print(f"Renderer: Closed window {window_id}.")
                self.render_all_windows()
                return {"type": "Success", "payload": {"window_id": window_id}}
            else:
                print(f"Renderer: Error - Window {window_id} not found for CloseWindow.")
                return {"type": "Error", "payload": {"message": f"Window {window_id} not found"}}

        elif req_type == "GetWindows":
            window_infos = []
            for win_id, win in self.windows.items():
                window_infos.append({
                    "id": win.id,
                    "title": win.title,
                    "x": 0, # Simplified, actual pos not tracked by MockWindow
                    "y": 0,
                    "width": win.width,
                    "height": win.height,
                })
            return {"type": "Windows", "payload": window_infos}

        else:
            print(f"Renderer: Unknown request type: {req_type}")
            return {"type": "Error", "payload": {"message": f"Unknown request type: {req_type}"}}

    def render_all_windows(self):
        if not self.windows:
            clear_output(wait=True)
            print("No windows to display.")
            return

        with BytesIO() as buffer:
            # For simplicity, just display the first window that has been drawn to
            # In a real scenario, you'd composite all windows onto a single canvas.
            first_drawn_window = None
            for window in self.windows.values():
                # Check if the framebuffer actually contains non-zero pixel data (i.e., something was drawn)
                if np.any(window.framebuffer != 0):
                    first_drawn_window = window
                    break
            
            if first_drawn_window:
                img = first_drawn_window.get_image()
                img.save(buffer, format="PNG")
                buffer.seek(0)
                clear_output(wait=True)
                display(IPImage(data=buffer.getvalue()))
            else:
                clear_output(wait=True)
                print("Windows created, but no drawing operations yet.")


# Global renderer instance
renderer = ColabUIRenderer()

async def process_ipc_message(message):
    return await renderer.handle_request(message)

# Helper to trigger initial display if needed
def start_ui_renderer():
    print("Colab UI Renderer ready to process IPC messages.")
    renderer.render_all_windows() # Initial empty display

# Call this from your notebook to start the mock renderer loop
# asyncio.create_task(start_ui_renderer())
