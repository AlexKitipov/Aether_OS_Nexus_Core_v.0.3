#![no_std]

use serde::{Deserialize, Serialize};
use alloc::string::String;

#[derive(Debug, Serialize, Deserialize)]
pub enum EchoRequest {
    Echo { message: String },
}

#[derive(Debug, Serialize, Deserialize)]
pub enum EchoResponse {
    EchoReply { message: String },
    Error(String),
}
