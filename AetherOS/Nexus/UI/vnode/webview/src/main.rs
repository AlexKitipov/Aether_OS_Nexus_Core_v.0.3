#![no_std]
#![no_main]
#![feature(alloc_error_handler)]

#[cfg(feature = "alloc")]
extern crate alloc;

#[cfg(feature = "serde")]
extern crate serde;

use alloc::collections::BTreeMap;
use alloc::format;
use alloc::string::String;
use alloc::vec;
use alloc::vec::Vec;
use core::panic::PanicInfo;
use linked_list_allocator::LockedHeap;

use aetheros_common::ipc::keyboard_ipc::KeyEvent;
use aetheros_common::ipc::vnode::VNodeChannel;
use aetheros_common::ipc::webview::{WebViewCommand, WebViewResponse};
use aetheros_common::swarm_engine::{SwarmEngine, SwarmTransport};
use aetheros_common::syscall::{syscall3, SYS_LOG, SUCCESS};
use aetheros_common::trust::{Aid, TrustStore};
use aetheros_common::ui::css_engine::CssEngine;
use aetheros_common::ui::html_parser::{DomNode, HtmlParser};

const VNODE_HEAP_SIZE: usize = 64 * 1024;
static mut VNODE_HEAP: [u8; VNODE_HEAP_SIZE] = [0; VNODE_HEAP_SIZE];

#[global_allocator]
static ALLOCATOR: LockedHeap = LockedHeap::empty();

#[alloc_error_handler]
fn alloc_error_handler(_layout: core::alloc::Layout) -> ! {
    loop {}
}

fn init_allocator() {
    unsafe {
        ALLOCATOR.lock().init(VNODE_HEAP.as_mut_ptr(), VNODE_HEAP_SIZE);
    }
}

fn log(msg: &str) {
    let _ = syscall3(SYS_LOG, msg.as_ptr() as u64, msg.len() as u64, 0);
}

fn update_input_buffer(buffer: &mut String, event: KeyEvent) {
    if event.ascii == Some(8) {
        let _ = buffer.pop();
        return;
    }

    if event.ascii == Some(b'\n') {
        buffer.clear();
        return;
    }

    if let Some(ch) = event.ascii {
        buffer.push(ch as char);
    }
}

fn extract_text(node: &DomNode) -> String {
    match node {
        DomNode::Text(text) => text.clone(),
        DomNode::Element { children, .. } => {
            let mut collected = String::new();
            for child in children {
                let piece = extract_text(child);
                if piece.is_empty() {
                    continue;
                }
                if !collected.is_empty() {
                    collected.push('\n');
                }
                collected.push_str(&piece);
            }
            collected
        }
    }
}

fn render_mail_preview(message_id: u32, html_body: String, css: Option<String>) -> WebViewResponse {
    let parser = HtmlParser::new();
    let css_engine = CssEngine::new();

    let dom = parser.parse_html(&html_body);
    let css_rules = css
        .as_deref()
        .map(|stylesheet| css_engine.parse_css(stylesheet))
        .unwrap_or_default();

    let applied_styles: BTreeMap<String, String> = css_engine.apply_styles(&dom, &css_rules);
    let extracted_text = extract_text(&dom);

    WebViewResponse::RenderedMail {
        message_id,
        extracted_text,
        applied_styles,
    }
}

fn main() -> ! {
    let mut channel = VNodeChannel::new(12);
    let _trust_store = TrustStore::new();
    let _aid = Aid([1; 32]);
    let _swarm_engine = SwarmEngine;
    let _swarm_transport = SwarmTransport;

    let _framebuffer: Vec<u8> = vec![0; 4];
    let status: String = format!("webview placeholder started (SUCCESS={})", SUCCESS);
    log(&status);

    let mut input_buffer = String::new();
    loop {
        if let Ok(Some(message)) = channel.recv_non_blocking() {
            match postcard::from_bytes::<WebViewCommand>(&message) {
                Ok(WebViewCommand::InjectKeyEvent { event }) => {
                    update_input_buffer(&mut input_buffer, event);
                    log(&format!(
                        "webview: key scancode=0x{:02x} ascii={:?} input='{}'",
                        event.scancode, event.ascii, input_buffer
                    ));
                    let _ = channel.send(&WebViewResponse::Ack);
                }
                Ok(WebViewCommand::Navigate { url }) => {
                    log(&format!("webview: navigate request '{}'", url));
                    let _ = channel.send(&WebViewResponse::Ack);
                }
                Ok(WebViewCommand::RenderMailMessage {
                    message_id,
                    html_body,
                    css,
                }) => {
                    let response = render_mail_preview(message_id, html_body, css);
                    let _ = channel.send(&response);
                }
                Err(_) => {
                    log("webview: failed to decode command payload.");
                    let _ = channel.send(&WebViewResponse::Error {
                        message: String::from("failed to decode command payload"),
                    });
                }
            }
        }
    }
}

#[no_mangle]
pub extern "C" fn _start() -> ! {
    init_allocator();
    main()
}

#[panic_handler]
fn panic(_info: &PanicInfo) -> ! {
    loop {}
}
