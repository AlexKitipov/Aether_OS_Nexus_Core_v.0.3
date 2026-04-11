extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::string::String;

use crate::ipc::keyboard_ipc::KeyEvent;

/// Commands accepted by the WebView V-Node.
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum WebViewCommand {
    /// Streams keyboard input events into the currently focused document/input field.
    InjectKeyEvent { event: KeyEvent },
    /// Navigates the active page to a URL.
    Navigate { url: String },
    /// Renders the supplied mail message payload as HTML/CSS-aware content.
    RenderMailMessage {
        message_id: u32,
        html_body: String,
        css: Option<String>,
    },
}

/// Replies emitted by the WebView V-Node.
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum WebViewResponse {
    Ack,
    RenderedMail {
        message_id: u32,
        extracted_text: String,
        applied_styles: BTreeMap<String, String>,
    },
    Error { message: String },
}
