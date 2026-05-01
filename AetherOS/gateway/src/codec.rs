use crate::protocol::GatewayMessage;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum CodecError {
    #[error("serialization error: {0}")]
    Serialization(#[from] serde_json::Error),
}

pub fn encode(message: &GatewayMessage) -> Result<Vec<u8>, CodecError> {
    serde_json::to_vec(message).map_err(CodecError::from)
}

pub fn decode(bytes: &[u8]) -> Result<GatewayMessage, CodecError> {
    serde_json::from_slice(bytes).map_err(CodecError::from)
}
