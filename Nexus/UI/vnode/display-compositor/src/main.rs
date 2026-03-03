// vnode/display-compositor/src/main.rs

#![no_std]
#![no_main]

extern crate alloc;

use core::panic::PanicInfo;
use alloc::vec::Vec;
use alloc::collections::BTreeMap;
use alloc::format;
use alloc::string::{String, ToString};

use common::ipc::vnode::VNodeChannel;
use common::syscall::{syscall3, SYS_LOG, SUCCESS, SYS_TIME};
use common::ui_protocol::{UiRequest, UiResponse, WindowInfo, MouseEventType, KeyEventType};

// Temporary log function for V-Nodes
fn log(msg: &str) {
    unsafe {
        let res = syscall3(
            SYS_LOG,
            msg.as_ptr() as u64,
            msg.len() as u64,
            0 // arg3 is unused for SYS_LOG
        );
        if res != SUCCESS { /* Handle log error, maybe panic or fall back */ }
    }
}

struct WindowSurface {
    id: u32,
    title: String,
    x: u32,
    y: u32,
    width: u32,
    height: u32,
    // In a real system, this would point to a shared memory region for the framebuffer
    // For this stub, we'll just acknowledge the pixels.
}

struct DisplayCompositor {
    client_chan: VNodeChannel, // Channel for communication with client UI V-Nodes
    next_window_id: u32,
    windows: BTreeMap<u32, WindowSurface>,
}

impl DisplayCompositor {
    fn new(client_chan_id: u32) -> Self {
        let client_chan = VNodeChannel::new(client_chan_id);
        log("Display Compositor: Initializing...");

        Self {
            client_chan,
            next_window_id: 1,
            windows: BTreeMap::new(),
        }
    }

    fn handle_request(&mut self, request: UiRequest) -> UiResponse {
        match request {
            UiRequest::CreateWindow { title, width, height } => {
                let id = self.next_window_id;
                self.next_window_id += 1;

                let new_window = WindowSurface { id, title: title.clone(), x: 0, y: 0, width, height };
                self.windows.insert(id, new_window);

                log(&alloc::format!("Display Compositor: Created window '{}' with ID: {}.", title, id));
                UiResponse::Success { window_id: Some(id) }
            },
            UiRequest::DrawToSurface { window_id, x, y, width, height, pixels } => {
                if let Some(window) = self.windows.get(&window_id) {
                    log(&alloc::format!("Display Compositor: Drawing to window {} at ({},{}) with size {}x{}. Pixel data length: {}.",
                        window_id, x, y, width, height, pixels.len()));
                    // In a real system, this would blit `pixels` to the framebuffer at the correct position.
                    UiResponse::Success { window_id: Some(window_id) }
                } else {
                    log(&alloc::format!("Display Compositor: DrawToSurface failed, window {} not found.", window_id));
                    UiResponse::Error { message: alloc::format!("Window {} not found.", window_id) }
                }
            },
            UiRequest::MouseEvent { window_id, x, y, button, event_type } => {
                log(&alloc::format!("Display Compositor: Mouse event {:?} on window {} at ({},{}) button {}.", event_type, window_id, x, y, button));
                // In a real system, this would route the event to the appropriate V-Node (e.g., focused window).
                UiResponse::Success { window_id: Some(window_id) }
            },
            UiRequest::KeyEvent { window_id, keycode, event_type } => {
                log(&alloc::format!("Display Compositor: Keyboard event {:?} on window {} for keycode {}.", event_type, window_id, keycode));
                // In a real system, this would route the event to the appropriate V-Node.
                UiResponse::Success { window_id: Some(window_id) }
            },
            UiRequest::CloseWindow { window_id } => {
                if self.windows.remove(&window_id).is_some() {
                    log(&alloc::format!("Display Compositor: Closed window {}.", window_id));
                    UiResponse::Success { window_id: Some(window_id) }
                } else {
                    log(&alloc::format!("Display Compositor: CloseWindow failed, window {} not found.", window_id));
                    UiResponse::Error { message: alloc::format!("Window {} not found.", window_id) }
                }
            },
            UiRequest::GetWindows => {
                let window_infos: Vec<WindowInfo> = self.windows.values().map(|w| WindowInfo {
                    id: w.id,
                    title: w.title.clone(),
                    x: w.x,
                    y: w.y,
                    width: w.width,
                    height: w.height,
                }).collect();
                log(&alloc::format!("Display Compositor: Returning {} window infos.", window_infos.len()));
                UiResponse::Windows(window_infos)
            },
        }
    }

    fn run_loop(&mut self) -> ! {
        log("Display Compositor: Entering main event loop.");
        loop {
            // Process incoming requests from client UI V-Nodes
            if let Ok(Some(req_data)) = self.client_chan.recv_non_blocking() {
                if let Ok(request) = postcard::from_bytes::<UiRequest>(&req_data) {
                    log(&alloc::format!("Display Compositor: Received UiRequest: {:?}.", request));
                    let response = self.handle_request(request);
                    self.client_chan.send(&response).unwrap_or_else(|_| log("Display Compositor: Failed to send response to client."));
                } else {
                    log("Display Compositor: Failed to deserialize UiRequest.");
                }
            }

            // Yield to other V-Nodes to prevent busy-waiting
            unsafe { syscall3(SYS_TIME, 0, 0, 0); }
        }
    }
}

#[no_mangle]
pub extern "C" fn _start() -> ! {
    // Assuming channel ID 12 for UI Compositor communication
    let mut compositor_vnode = DisplayCompositor::new(12);
    compositor_vnode.run_loop();
}

#[panic_handler]
pub extern "C" fn panic(info: &PanicInfo) -> ! {
    log(&alloc::format!("Display Compositor V-Node panicked! Info: {:?}.", info));
    loop {}
}