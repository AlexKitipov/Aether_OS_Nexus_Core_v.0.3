//! Gateway protocol definitions for communication between
//! the Nexus kernel and the AI orchestrator.

use serde::{Deserialize, Serialize};

/// High-level message types exchanged through the gateway.
#[derive(Debug, Serialize, Deserialize)]
pub enum GatewayMessage {
    /// Kernel → AI: request for analysis or decision.
    KernelEvent {
        event_type: String,
        payload: Vec<u8>,
    },

    /// AI → Kernel: action or command to apply.
    AiAction {
        action_type: String,
        payload: Vec<u8>,
    },

    /// Generic error message.
    Error {
        message: String,
    },
}

/// Wrapper for responses from AI or kernel.
#[derive(Debug, Serialize, Deserialize)]
pub struct GatewayResponse {
    pub status: String,   // "ok" | "error"
    pub payload: Vec<u8>, // raw serialized data
}
