
// src/ipc/shell_ipc.rs

#![no_std]

extern crate alloc;
use alloc::vec::Vec;
use alloc::string::String;

use serde::{Deserialize, Serialize};

/// Represents requests from client V-Nodes (e.g., AetherTerminal, other V-Nodes) to the Shell V-Node.
#[derive(Debug, Serialize, Deserialize)]
pub enum ShellRequest {
    /// Request to execute a command with its arguments.
    ExecuteCommand { command: String, args: Vec<String> },
    /// Request to change the current working directory.
    ChangeDirectory { path: String },
    /// Request to get the current working directory.
    GetCurrentDirectory,
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
}
