// common/src/ipc/shell_ipc.rs


extern crate alloc;
use alloc::vec::Vec;
use alloc::string::String;

use serde::{Deserialize, Serialize};

pub use crate::logger_ipc::LogLevel;

/// Represents requests from client V-Nodes (e.g., AetherTerminal, other V-Nodes) to the Shell V-Node.
#[derive(Debug, Serialize, Deserialize)]
pub enum ShellRequest {
    /// Request to execute a command with its arguments.
    ExecuteCommand { command: String, args: Vec<String> },
    /// Request to change the current working directory.
    ChangeDirectory { path: String },
    /// Request to get the current working directory.
    GetCurrentDirectory,
    /// Request to run a command on the logger V-Node.
    RunLoggerCommand { message: String, level: LogLevel },
    /// Request to run a command on the echo V-Node.
    RunEchoCommand { message: String },
    /// Request to run a command on the test V-Node.
    RunTestCommand { test_name: String, args: Vec<String> },
}

/// Represents responses from the Shell V-Node to client V-Nodes.
#[derive(Debug, Serialize, Deserialize)]
pub enum ShellResponse {
    /// Successful execution of a command, with its output and exit code.
    CommandOutput { stdout: String, stderr: String, exit_code: i32 },
    /// Indicates a successful operation without specific output.
    Success(String),
    /// Returns the current working directory.
    CurrentDirectory(String),
    /// Indicates an error occurred during the operation.
    Error(String),
    /// Response from a logger command.
    LoggerResult { success: bool },
    /// Response from an echo command.
    EchoResult { reply: String },
    /// Response from a test command.
    TestResult { stdout: String, stderr: String, success: bool },
}
