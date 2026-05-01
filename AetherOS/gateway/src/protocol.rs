use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum GatewayMessage {
    GetState { scope: String },
    ApplyAction { action: String, payload: Vec<u8> },
    Log { level: String, message: String },
    Ack { request_id: u64, ok: bool, details: String },
}
