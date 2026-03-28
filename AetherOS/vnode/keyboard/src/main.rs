#![no_std]
#![no_main]

extern crate alloc;

use alloc::format;
use alloc::string::String;
use core::panic::PanicInfo;
use linked_list_allocator::LockedHeap;

use common::ipc::init_ipc::InitRequest;
use common::ipc::keyboard_ipc::KeyEvent;
use common::ipc::logger_ipc::{LogLevel, LoggerRequest};
use common::ipc::model_runtime_ipc::{InferRequest, InferResponse};
use common::ipc::shell_ipc::ShellRequest;
use common::ipc::webview::WebViewCommand;
use common::ipc::vnode::VNodeChannel;
use common::ipc::IpcSend;
use common::syscall::{
    syscall3, syscall_log, E_ERROR, SUCCESS, SYS_IPC_RECV, SYS_IRQ_ACK, SYS_IRQ_REGISTER,
};

const VNODE_HEAP_SIZE: usize = 64 * 1024;
static mut VNODE_HEAP: [u8; VNODE_HEAP_SIZE] = [0; VNODE_HEAP_SIZE];

const KEYBOARD_IRQ: u64 = 1;
const KEYBOARD_IRQ_CHANNEL_ID: u32 = 4;
const SYSTEM_INPUT_CHANNEL_ID: u32 = 5;
const SHELL_COMMAND_CHANNEL_ID: u32 = 8;
const WEBVIEW_CHANNEL_ID: u32 = 12;
const LOGGER_CHANNEL_ID: u32 = 2;
const INIT_SERVICE_CHANNEL_ID: u32 = 1;
const MODEL_RUNTIME_CHANNEL_ID: u32 = 11;

const AUTOCOMPLETE_MIN_PROMPT_LEN: usize = 4;
const AUTOCOMPLETE_MAX_PROMPT_LEN: usize = 64;

#[global_allocator]
static GLOBAL_ALLOCATOR: LockedHeap = LockedHeap::empty();

fn init_allocator() {
    unsafe {
        GLOBAL_ALLOCATOR
            .lock()
            .init(VNODE_HEAP.as_mut_ptr(), VNODE_HEAP_SIZE);
    }
}

fn log(msg: &str) {
    let mut logger_chan = VNodeChannel::new(LOGGER_CHANNEL_ID);
    let _ = logger_chan.send(&LoggerRequest::Log {
        message: format!("[keyboard] {}", msg),
        level: LogLevel::Info,
    });

    // Best-effort fallback to kernel SYS_LOG for early bring-up scenarios.
    let _ = syscall_log(msg);
}

fn translate_scancode(scancode: u8) -> Option<u8> {
    match scancode {
        0x02 => Some(b'1'),
        0x03 => Some(b'2'),
        0x04 => Some(b'3'),
        0x05 => Some(b'4'),
        0x06 => Some(b'5'),
        0x07 => Some(b'6'),
        0x08 => Some(b'7'),
        0x09 => Some(b'8'),
        0x0A => Some(b'9'),
        0x0B => Some(b'0'),
        0x10 => Some(b'q'),
        0x11 => Some(b'w'),
        0x12 => Some(b'e'),
        0x13 => Some(b'r'),
        0x14 => Some(b't'),
        0x15 => Some(b'y'),
        0x16 => Some(b'u'),
        0x17 => Some(b'i'),
        0x18 => Some(b'o'),
        0x19 => Some(b'p'),
        0x1E => Some(b'a'),
        0x1F => Some(b's'),
        0x20 => Some(b'd'),
        0x21 => Some(b'f'),
        0x22 => Some(b'g'),
        0x23 => Some(b'h'),
        0x24 => Some(b'j'),
        0x25 => Some(b'k'),
        0x26 => Some(b'l'),
        0x2C => Some(b'z'),
        0x2D => Some(b'x'),
        0x2E => Some(b'c'),
        0x2F => Some(b'v'),
        0x30 => Some(b'b'),
        0x31 => Some(b'n'),
        0x32 => Some(b'm'),
        0x39 => Some(b' '),
        0x0E => Some(8), // backspace
        0x1C => Some(b'\n'),
        _ => None,
    }
}

fn translate_scancode_with_modifiers(
    scancode: u8,
    shift_active: bool,
    caps_lock_active: bool,
) -> Option<u8> {
    let base = translate_scancode(scancode)?;

    if base == b' ' || base == b'\n' || base == 8 {
        return Some(base);
    }

    if base.is_ascii_alphabetic() {
        let uppercase = shift_active ^ caps_lock_active;
        return Some(if uppercase {
            base.to_ascii_uppercase()
        } else {
            base
        });
    }

    if !shift_active {
        return Some(base);
    }

    let shifted = match base {
        b'1' => b'!',
        b'2' => b'@',
        b'3' => b'#',
        b'4' => b'$',
        b'5' => b'%',
        b'6' => b'^',
        b'7' => b'&',
        b'8' => b'*',
        b'9' => b'(',
        b'0' => b')',
        _ => base,
    };
    Some(shifted)
}

fn update_prompt(prompt: &mut String, ch: u8) {
    if ch == 8 {
        let _ = prompt.pop();
        return;
    }

    if ch == b'\n' {
        prompt.clear();
        return;
    }

    if prompt.len() >= AUTOCOMPLETE_MAX_PROMPT_LEN {
        prompt.remove(0);
    }

    prompt.push(ch as char);
}

fn parse_command_line(line: &str) -> Option<(String, alloc::vec::Vec<String>)> {
    let mut parts = line.split_whitespace();
    let command = parts.next()?;
    let args = parts.map(String::from).collect();
    Some((String::from(command), args))
}

fn dispatch_shell_command(command_chan: &mut VNodeChannel, line: &str) {
    if let Some((command, args)) = parse_command_line(line) {
        let request = ShellRequest::ExecuteCommand { command, args };
        if command_chan.send(&request).is_err() {
            log("keyboard: failed to dispatch ShellRequest::ExecuteCommand.");
        } else {
            log(&format!("keyboard: dispatched shell command='{}'", line));
        }
    }
}

fn maybe_request_autocomplete(model_chan: &mut VNodeChannel, prompt: &str) {
    if prompt.len() < AUTOCOMPLETE_MIN_PROMPT_LEN || prompt.ends_with(' ') {
        return;
    }

    let request = InferRequest::TextGeneration {
        model_id: String::from("tiny-autocomplete"),
        prompt: String::from(prompt),
        max_tokens: 12,
    };

    if model_chan.send(&request).is_err() {
        log("keyboard: failed to send autocomplete request to model-runtime.");
        return;
    }

}

fn poll_autocomplete_response(model_chan: &mut VNodeChannel) {
    if let Ok(Some(response_bytes)) = model_chan.recv_non_blocking() {
        match postcard::from_bytes::<InferResponse>(&response_bytes) {
            Ok(InferResponse::TextGenerationResult { generated_text }) => {
                log(&format!("autocomplete suggestion='{}'", generated_text));
            }
            Ok(InferResponse::Error { message }) => {
                log(&format!("autocomplete runtime error: {}", message));
            }
            Ok(_) => {
                log("keyboard: unexpected model-runtime response variant.");
            }
            Err(_) => {
                log("keyboard: failed to decode model-runtime response.");
            }
        }
    }
}

fn decode_scancode(raw: &[u8], recv_len: u64) -> Option<u8> {
    let recv_len = recv_len as usize;
    if recv_len == 0 || recv_len > raw.len() {
        return None;
    }

    if let Ok(event) = postcard::from_bytes::<KeyEvent>(&raw[..recv_len]) {
        return Some(event.scancode);
    }

    Some(raw[0])
}

fn ack_keyboard_irq() {
    unsafe {
        let ack_res = syscall3(SYS_IRQ_ACK, KEYBOARD_IRQ, 0, 0);
        if ack_res != SUCCESS {
            log(&format!("Keyboard V-Node failed to ACK IRQ1: {}", ack_res));
        }
    }
}

#[no_mangle]
pub extern "C" fn _start() -> ! {
    init_allocator();
    let irq_chan = VNodeChannel::new(KEYBOARD_IRQ_CHANNEL_ID);
    let mut input_chan = VNodeChannel::new(SYSTEM_INPUT_CHANNEL_ID);
    let mut shell_command_chan = VNodeChannel::new(SHELL_COMMAND_CHANNEL_ID);
    let mut webview_chan = VNodeChannel::new(WEBVIEW_CHANNEL_ID);
    let mut model_runtime_chan = VNodeChannel::new(MODEL_RUNTIME_CHANNEL_ID);

    let mut init_chan = VNodeChannel::new(INIT_SERVICE_CHANNEL_ID);
    let _ = init_chan.send(&InitRequest::ServiceReady {
        service_name: format!("vnode.keyboard"),
        pid: None,
    });

    unsafe {
        let res = syscall3(SYS_IRQ_REGISTER, KEYBOARD_IRQ, irq_chan.id as u64, 0);
        if res != SUCCESS {
            log(&format!("Keyboard V-Node failed to register IRQ1: {}", res));
            panic!("IRQ1 registration failed");
        }
    }

    log("Keyboard V-Node started and IRQ1 registered.");

    let mut raw = [0u8; 8];
    let mut prompt = String::new();
    let mut shift_active = false;
    let mut caps_lock_active = false;
    loop {
        let recv_len = unsafe {
            syscall3(
                SYS_IPC_RECV,
                irq_chan.id as u64,
                raw.as_mut_ptr() as u64,
                raw.len() as u64,
            )
        };

        if recv_len == 0 || recv_len == E_ERROR {
            continue;
        }

        let Some(scancode) = decode_scancode(&raw, recv_len) else {
            ack_keyboard_irq();
            continue;
        };

        match scancode {
            // Left Shift / Right Shift pressed.
            0x2A | 0x36 => {
                shift_active = true;
                ack_keyboard_irq();
                continue;
            }
            // Left Shift / Right Shift released.
            0xAA | 0xB6 => {
                shift_active = false;
                ack_keyboard_irq();
                continue;
            }
            // Caps Lock toggled on key press.
            0x3A => {
                caps_lock_active = !caps_lock_active;
                log(&format!(
                    "keyboard: caps_lock_active={}",
                    caps_lock_active
                ));
                ack_keyboard_irq();
                continue;
            }
            _ => {}
        }

        // Ignore key-release events for non-modifier keys.
        if (scancode & 0x80) != 0 {
            ack_keyboard_irq();
            continue;
        }

        let ascii = translate_scancode_with_modifiers(scancode, shift_active, caps_lock_active);

        if let Some(ch) = ascii {
            let key_event = KeyEvent::new(scancode, Some(ch));
            if input_chan.send(&key_event).is_err() {
                log(&format!(
                    "keyboard: failed to forward key event to input channel {}.",
                    SYSTEM_INPUT_CHANNEL_ID
                ));
            }
            if webview_chan
                .send(&WebViewCommand::InjectKeyEvent { event: key_event })
                .is_err()
            {
                log("keyboard: failed to forward key event to webview.");
            }
            let command_to_dispatch = if ch == b'\n' {
                let completed = prompt.trim().to_string();
                update_prompt(&mut prompt, ch);
                if completed.is_empty() {
                    None
                } else {
                    Some(completed)
                }
            } else {
                update_prompt(&mut prompt, ch);
                None
            };

            if let Some(command_line) = command_to_dispatch {
                dispatch_shell_command(&mut shell_command_chan, &command_line);
            } else {
                maybe_request_autocomplete(&mut model_runtime_chan, &prompt);
                poll_autocomplete_response(&mut model_runtime_chan);
            }
            log(&format!("keyboard: scancode=0x{:02x} ascii='{}'", scancode, ch as char));
        } else {
            let key_event = KeyEvent::new(scancode, None);
            if input_chan.send(&key_event).is_err() {
                log(&format!(
                    "keyboard: failed to forward non-printable key event to channel {}.",
                    SYSTEM_INPUT_CHANNEL_ID
                ));
            }
            if webview_chan
                .send(&WebViewCommand::InjectKeyEvent { event: key_event })
                .is_err()
            {
                log("keyboard: failed to forward non-printable key event to webview.");
            }
            log(&format!("keyboard: scancode=0x{:02x}", scancode));
        }

        ack_keyboard_irq();
    }
}

#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    log(&format!("Keyboard V-Node panic: {:?}", info));
    loop {}
}
