// vnode/webview/src/main.rs

#![no_std]
#![no_main]

extern crate alloc;

use core::panic::PanicInfo;
use alloc::vec::Vec;
use alloc::format;
use alloc::string::{String, ToString};

use common::ipc::vnode::VNodeChannel;
use common::syscall::{syscall3, SYS_LOG, SUCCESS, SYS_TIME};
use common::ui_protocol::{UiRequest, UiResponse, WindowInfo, MouseEventType, KeyEventType};
use common::ui::{HtmlParser, CssEngine, LayoutEngine};
use common::ui::html_parser::DomNode;

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

struct WebViewVNode {
    client_chan: VNodeChannel, // Channel for communication with UI Compositor
    html_parser: HtmlParser,
    css_engine: CssEngine,
    layout_engine: LayoutEngine,
    window_id: Option<u32>,
}

impl WebViewVNode {
    fn new(client_chan_id: u32) -> Self {
        let client_chan = VNodeChannel::new(client_chan_id);
        log("WebView V-Node: Initializing...");

        Self {
            client_chan,
            html_parser: HtmlParser::new(),
            css_engine: CssEngine::new(),
            layout_engine: LayoutEngine::new(),
            window_id: None,
        }
    }

    fn run_loop(&mut self) -> ! {
        log("WebView V-Node: Entering main event loop.");

        // 1. Request to create a window
        let create_window_req = UiRequest::CreateWindow {
            title: String::from("AetherOS WebView"),
            width: 800,
            height: 600,
        };

        match self.client_chan.send_and_recv(&create_window_req) {
            Ok(UiResponse::Success { window_id: Some(id) }) => {
                self.window_id = Some(id);
                log(&alloc::format!("WebView: Created window with ID: {}.", id));
            },
            Ok(UiResponse::Error { message }) => {
                log(&alloc::format!("WebView: Failed to create window: {}. Panicking.", message));
                panic!("Failed to create window");
            },
            _ => {
                log("WebView: Unexpected response for CreateWindow. Panicking.");
                panic!("Unexpected CreateWindow response");
            }
        }

        // 2. Simulate loading an HTML page
        let html_content = "<html><body>Hello from WebView!</body></html>";
        let css_content = "body { background-color: white; color: black; }";

        log(&alloc::format!("WebView: Parsing HTML: {}", html_content));
        let dom_tree = self.html_parser.parse_html(html_content);
        log(&alloc::format!("WebView: Parsed DOM: {:?}", dom_tree));

        log(&alloc::format!("WebView: Parsing CSS: {}", css_content));
        let css_rules = self.css_engine.parse_css(css_content);
        log(&alloc::format!("WebView: Parsed CSS rules: {:?}", css_rules));

        let computed_styles = self.css_engine.apply_styles(&dom_tree, &css_rules);
        log(&alloc::format!("WebView: Computed styles: {:?}", computed_styles));

        // 3. Perform layout
        let layout_tree = self.layout_engine.layout(&dom_tree, &computed_styles, 800, 600);
        log(&alloc::format!("WebView: Computed layout: {:?}", layout_tree));

        // 4. Simulate rendering to a pixel buffer
        let mut pixels: Vec<u8> = Vec::new();
        pixels.resize(800 * 600 * 4, 0); // RGBA
        // For simplicity, just fill with a color based on the body background
        if let Some(bg_color) = computed_styles.get("background-color") {
            let color_val = match bg_color.as_str() {
                "white" => [0xFF, 0xFF, 0xFF, 0xFF],
                "black" => [0x00, 0x00, 0x00, 0xFF],
                _ => [0x80, 0x80, 0x80, 0xFF], // Gray default
            };
            for i in (0..pixels.len()).step_by(4) {
                pixels[i] = color_val[0];
                pixels[i+1] = color_val[1];
                pixels[i+2] = color_val[2];
                pixels[i+3] = color_val[3];
            }
        }

        // 5. Send pixel buffer to the UI Compositor
        if let Some(id) = self.window_id {
            let draw_req = UiRequest::DrawToSurface {
                window_id: id,
                x: 0,
                y: 0,
                width: 800,
                height: 600,
                pixels,
            };

            match self.client_chan.send_and_recv(&draw_req) {
                Ok(UiResponse::Success { .. }) => {
                    log(&alloc::format!("WebView: Sent rendered frame to compositor for window {}.", id));
                },
                Ok(UiResponse::Error { message }) => {
                    log(&alloc::format!("WebView: Failed to draw to surface: {}. Panicking.", message));
                    panic!("Failed to draw to surface");
                },
                _ => {
                    log("WebView: Unexpected response for DrawToSurface. Panicking.");
                    panic!("Unexpected DrawToSurface response");
                }
            }
        }

        loop {
            // WebView V-Node would typically idle here, waiting for UI events (mouse, keyboard) or navigation requests.
            // For now, it just yields.
            unsafe { syscall3(SYS_TIME, 0, 0, 0); }
        }
    }
}

#[no_mangle]
pub extern "C" fn _start() -> ! {
    // Assuming channel ID 12 for UI Compositor communication
    let mut webview_vnode = WebViewVNode::new(12);
    webview_vnode.run_loop();
}

#[panic_handler]
pub extern "C" fn panic(info: &PanicInfo) -> ! {
    log(&alloc::format!("WebView V-Node panicked! Info: {:?}.", info));
    loop {}
}