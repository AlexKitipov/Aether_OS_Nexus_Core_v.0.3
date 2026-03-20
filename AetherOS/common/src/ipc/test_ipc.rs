
use serde::{Deserialize, Serialize};
use alloc::string::String;

pub use crate::ipc::logger_ipc::LogLevel;

#[derive(Debug, Serialize, Deserialize)]
pub enum TestRequest {
    RunEchoTest { message: String },
    RunLoggerTest { message: String, level: LogLevel },
}

#[derive(Debug, Serialize, Deserialize)]
pub enum TestResponse {
    EchoResult { reply: String },
    LoggerResult { success: bool },
    Error(String),
}
