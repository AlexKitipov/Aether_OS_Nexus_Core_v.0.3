
// src/ipc/vfs_ipc.rs

#![no_std]

extern crate alloc;
use alloc::vec::Vec;
use alloc::string::String;
use alloc::collections::BTreeMap;

use serde::{Deserialize, Serialize};

// Placeholder for File Descriptor type
pub type Fd = u32;

// Placeholder for VFS metadata structure
#[derive(Debug, Serialize, Deserialize)]
pub struct VfsMetadata {
    pub is_dir: bool,
    pub size: u64,
    pub created: u64, // Unix timestamp
    pub modified: u64,
    pub permissions: u32, // e.g., 0o755
    // Add more fields as needed
}

/// Represents requests from client V-Nodes to the VFS V-Node.
#[derive(Debug, Serialize, Deserialize)]
pub enum VfsRequest {
    /// Open a file or directory.
    Open { path: String, flags: u32 }, // flags could be O_RDONLY, O_WRONLY, O_CREAT, etc.
    /// Read from an open file descriptor.
    Read { fd: Fd, len: u32, offset: u64 },
    /// Write to an open file descriptor.
    Write { fd: Fd, data: Vec<u8>, offset: u64 },
    /// List contents of a directory (given its path).
    List { path: String },
    /// Get metadata about a file or directory.
    Stat { path: String },
    /// Close an open file descriptor.
    Close { fd: Fd },
}

/// Represents responses from the VFS V-Node to client V-Nodes.
#[derive(Debug, Serialize, Deserialize)]
pub enum VfsResponse {
    /// Indicates a successful operation, optionally with a return value (e.g., new Fd).
    Success(i32), // Typically 0 for success, or a new Fd
    /// Returns data read from a file.
    Data(Vec<u8>),
    /// Returns metadata for a file or directory.
    Metadata(VfsMetadata),
    /// Returns a list of directory entries (name, metadata).
    DirectoryEntries(BTreeMap<String, VfsMetadata>),
    /// Indicates an error occurred.
    Error { code: i32, message: String }, // errno-like code and descriptive message
}
