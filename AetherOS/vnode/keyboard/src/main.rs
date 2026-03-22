#![no_std]
#![no_main]

extern crate alloc;

use alloc::format;
use alloc::string::String;
use core::panic::PanicInfo;
use linked_list_allocator::LockedHeap;

use common::ipc::init_ipc::InitRequest;
use common::ipc::logger_ipc::{LogLevel, LoggerRequest};
use common::ipc::model_runtime_ipc::{InferRequest, InferResponse};
use common::ipc::vnode::VNodeChannel;
use common::ipc::IpcSend;
use common::syscall::{
    syscall3, E_ERROR, SUCCESS, SYS_IPC_SEND, SYS_IPC_RECV, SYS_IRQ_ACK, SYS_IRQ_REGISTER, SYS_LOG,
};

const VNODE_HEAP_SIZE: usize = 64 * 1024;
static mut VNODE_HEAP: [u8; VNODE_HEAP_SIZE] = [0; VNODE_HEAP_SIZE];

const KEYBOARD_IRQ: u64 = 1;
const KEYBOARD_IRQ_CHANNEL_ID: u32 = 4;
const SYSTEM_INPUT_CHANNEL_ID: u32 = 5;
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
    unsafe {
        let _ = syscall3(SYS_LOG, msg.as_ptr() as u64, msg.len() as u64, 0);
    }
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

#[no_mangle]
pub extern "C" fn _start() -> ! {
    init_allocator();
    let irq_chan = VNodeChannel::new(KEYBOARD_IRQ_CHANNEL_ID);
    let mut model_runtime_chan = VNodeChannel::new(MODEL_RUNTIME_CHANNEL_ID);

    let mut init_chan = VNodeChannel::new(INIT_SERVICE_CHANNEL_ID);
    let _ = init_chan.send(&InitRequest::ServiceStatus {
        service_name: format!("vnode.keyboard"),
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

        let scancode = raw[0];
        let ascii = translate_scancode(scancode);

        if let Some(ch) = ascii {
            let payload = [scancode, ch];
            unsafe {
                let send_res = syscall3(
                    SYS_IPC_SEND,
                    SYSTEM_INPUT_CHANNEL_ID as u64,
                    payload.as_ptr() as u64,
                    payload.len() as u64,
                );
                if send_res != SUCCESS {
                    log(&format!(
                        "keyboard: failed to forward key event to input channel {} (code {}).",
                        SYSTEM_INPUT_CHANNEL_ID, send_res
                    ));
                }
            }
            update_prompt(&mut prompt, ch);
            maybe_request_autocomplete(&mut model_runtime_chan, &prompt);
            poll_autocomplete_response(&mut model_runtime_chan);
            log(&format!("keyboard: scancode=0x{:02x} ascii='{}'", scancode, ch as char));
        } else {
            log(&format!("keyboard: scancode=0x{:02x}", scancode));
        }

        unsafe {
            let ack_res = syscall3(SYS_IRQ_ACK, KEYBOARD_IRQ, 0, 0);
            if ack_res != SUCCESS {
                log(&format!("Keyboard V-Node failed to ACK IRQ1: {}", ack_res));
            }
        }
    }
}

#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    log(&format!("Keyboard V-Node panic: {:?}", info));
    loop {}
}
