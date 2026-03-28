extern crate alloc;

use alloc::string::String;

use serde::{Deserialize, Serialize};

use crate::ipc::keyboard_ipc::KeyEvent;

/// Commands accepted by the WebView V-Node.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum WebViewCommand {
    /// Streams keyboard input events into the currently focused document/input field.
    InjectKeyEvent { event: KeyEvent },
    /// Navigates the active page to a URL.
    Navigate { url: String },
}

/// Replies emitted by the WebView V-Node.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum WebViewResponse {
    Ack,
    Error { message: String },
}
