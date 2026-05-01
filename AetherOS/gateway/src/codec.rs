//! Serialization and deserialization utilities for the
//! GatewayMessage protocol.

use crate::protocol::{GatewayMessage, GatewayResponse};

/// Encode a GatewayMessage into bytes.
pub fn encode_message(msg: &GatewayMessage) -> Result<Vec<u8>, String> {
    serde_json::to_vec(msg).map_err(|e| e.to_string())
}

/// Decode bytes into a GatewayMessage.
pub fn decode_message(data: &[u8]) -> Result<GatewayMessage, String> {
    serde_json::from_slice(data).map_err(|e| e.to_string())
}

/// Encode a GatewayResponse into bytes.
pub fn encode_response(resp: &GatewayResponse) -> Result<Vec<u8>, String> {
    serde_json::to_vec(resp).map_err(|e| e.to_string())
}

/// Decode bytes into a GatewayResponse.
pub fn decode_response(data: &[u8]) -> Result<GatewayResponse, String> {
    serde_json::from_slice(data).map_err(|e| e.to_string())
}
