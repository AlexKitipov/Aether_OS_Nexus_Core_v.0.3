
#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};
use alloc::string::String;

#[derive(Debug)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub enum LoggerRequest {
    Log { message: String, level: LogLevel },
}

#[derive(Debug)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub enum LoggerResponse {
    Success,
    Error(String),
}

#[derive(Debug)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub enum LogLevel {
    Trace,
    Debug,
    Info,
    Warn,
    Error,
    Fatal,
}
