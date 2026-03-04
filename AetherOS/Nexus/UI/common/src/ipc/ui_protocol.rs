// common/src/ipc/ui_protocol.rs

#![no_std]

extern crate alloc;
use alloc::string::String;
use alloc::vec::Vec;

use serde::{Deserialize, Serialize};

/// Represents requests from client V-Nodes to the UI Compositor or other UI services.
#[derive(Debug, Serialize, Deserialize)]
pub enum UiRequest {
    /// Request to create a new window surface.
    CreateWindow {
        title: String,
        width: u32,
        height: u32,
    },
    /// Request to draw pixels to a specific window surface.
    DrawToSurface {
        window_id: u32,
        x: u32,
        y: u32,
        width: u32,
        height: u32,
        pixels: Vec<u8>, // RGBA pixel data
    },
    /// Request to handle a mouse event.
    MouseEvent {
        window_id: u32,
        x: u32,
        y: u32,
        button: u8,
        event_type: MouseEventType,
    },
    /// Request to handle a keyboard event.
    KeyEvent {
        window_id: u3n,
        keycode: u16,
        event_type: KeyEventType,
    },
    /// Request to close a window.
    CloseWindow {
        window_id: u32,
    },
    /// Request to get information about active windows.
    GetWindows,
}

/// Represents responses from the UI Compositor or other UI services to client V-Nodes.
#[derive(Debug, Serialize, Deserialize)]
pub enum UiResponse {
    /// Indicates a successful operation, optionally with a window ID.
    Success {
        window_id: Option<u32>,
    },
    /// Returns a list of active windows and their properties.
    Windows(Vec<WindowInfo>),
    /// Indicates an error occurred during a UI operation.
    Error {
        message: String,
    },
}

#[derive(Debug, Serialize, Deserialize)]
pub enum MouseEventType {
    MouseDown,
    MouseUp,
    MouseMove,
    Scroll,
}

#[derive(Debug, Serialize, Deserialize)]
pub enum KeyEventType {
    KeyDown,
    KeyUp,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct WindowInfo {
    pub id: u32,
    pub title: String,
    pub x: u32,
    pub y: u32,
    pub width: u32,
    pub height: u32,
}
