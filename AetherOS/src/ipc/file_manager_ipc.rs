
// src/ipc/file_manager_ipc.rs

#![no_std]

extern crate alloc;
use alloc::vec::Vec;
use alloc::string::String;
use alloc::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::ipc::vfs_ipc::VfsMetadata; // Reusing VfsMetadata

/// Represents requests from client V-Nodes to the File Manager V-Node.
#[derive(Debug, Serialize, Deserialize)]
pub enum FileManagerRequest {
    /// Browse the contents of a directory.
    Browse { path: String },
    /// Copy a file or directory.
    Copy { source: String, destination: String },
    /// Move a file or directory.
    Move { source: String, destination: String },
    /// Delete a file or directory.
    Delete { path: String },
    /// Create a new directory.
    CreateDirectory { path: String },
}

/// Represents responses from the File Manager V-Node to client V-Nodes.
#[derive(Debug, Serialize, Deserialize)]
pub enum FileManagerResponse {
    /// Indicates a successful operation, with an optional descriptive message.
    Success(String),
    /// Indicates an error occurred during the operation.
    Error(String),
    /// Returns a list of directory entries (name, metadata).
    DirectoryEntries(BTreeMap<String, VfsMetadata>),
}
