
# Virtual File System (VFS) V-Node (svc://vfs)

## Overview

The `vfs` V-Node acts as the central point for all file system interactions within AetherOS. It provides a unified, abstract interface for client V-Nodes to perform file and directory operations, regardless of the underlying storage backend (e.g., AetherFS, RAM disk, block device). This design ensures modularity, security through capability-based access control, and simplifies client-side file handling.

## IPC Protocol

Communication with the `vfs` V-Node occurs via IPC, using the `VfsRequest` and `VfsResponse` enums defined in `src/ipc/vfs_ipc.rs`.

### Fd (File Descriptor)

`pub type Fd = u32;`

Represents a unique identifier for an open file or directory within the `vfs` V-Node, analogous to a traditional Unix file descriptor.

### VfsMetadata

```rust
#[derive(Debug, Serialize, Deserialize)]
pub struct VfsMetadata {
    pub is_dir: bool,
    pub size: u64,
    pub created: u64, // Unix timestamp
    pub modified: u64,
    pub permissions: u32, // e.g., 0o755
    // Add more fields as needed
}
```

This structure provides detailed information about a file or directory.

### VfsRequest Enum (Client -> vfs)

Client V-Nodes send these requests to `svc://vfs` to perform file system operations.

```rust
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
```

**Parameters:**

*   `path`: A `String` representing the absolute path to the file or directory.
*   `flags`: A `u32` representing open flags (e.g., read-only, write-only, create if not exists). (Conceptual for now, actual implementation would define specific values).
*   `fd`: The `Fd` returned by a successful `Open` request.
*   `len`: The maximum number of bytes to read.
*   `offset`: The byte offset from the beginning of the file for read/write operations.
*   `data`: A `Vec<u8>` containing the data to write.

### VfsResponse Enum (vfs -> Client)

`svc://vfs` sends these responses back to the client V-Node after processing a `VfsRequest`.

```rust
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
```

**Return Values:**

*   `Success(i32)`: A successful operation. The `i32` usually contains a new `Fd` (for `Open`), `0` for other successful operations, or the number of bytes written/read (though `Data` and `Metadata` are more specific).
*   `Data(Vec<u8>)`: The data read from a `Read` operation.
*   `Metadata(VfsMetadata)`: The metadata for a file or directory from a `Stat` operation.
*   `DirectoryEntries(BTreeMap<String, VfsMetadata>)`: A map of entry names to their metadata from a `List` operation.
*   `Error { code: i32, message: String }`: An error occurred. The `i32` contains an `errno`-like error code, and the `String` provides a human-readable message.

## Functionality

The `vfs` V-Node performs the following key functions:

1.  **Request Routing**: Receives `VfsRequest` messages and routes them to the appropriate underlying file system driver (e.g., `svc://aetherfs`, `svc://ramdisk-driver`).
2.  **File Descriptor Management**: Manages a table of open file descriptors, mapping them to internal handles of the actual storage backends.
3.  **Path Resolution**: Resolves symbolic links and relative paths to absolute paths before delegating to backends.
4.  **Security Enforcement**: Enforces capability-based access control based on the calling V-Node's granted capabilities (e.g., `StorageAccess: "/home"`).
5.  **Metadata Caching**: Caches frequently accessed file metadata to improve performance.
6.  **Error Handling**: Translates errors from underlying file systems into standardized `VfsResponse::Error` messages.

## Usage Examples

### Example 1: Opening and Reading a File

```rust
// Pseudocode for client V-Node

let mut vfs_chan = VNodeChannel::new(7); // IPC Channel to svc://vfs

// 1. Open a file for reading
let open_req = VfsRequest::Open { path: String::from("/etc/network/config.txt"), flags: 0 }; // 0 for O_RDONLY (conceptual)
let fd: Fd = match vfs_chan.send_and_recv::<VfsRequest, VfsResponse>(&open_req) {
    Ok(VfsResponse::Success(file_fd)) => {
        log!("File opened successfully with fd: {}", file_fd);
        file_fd as Fd
    },
    Ok(VfsResponse::Error { code, message }) => {
        log!("Failed to open file: {} ({})", message, code);
        return; // Handle error
    },
    _ => { log!("Unexpected response during file open."); return; }
};

// 2. Read from the opened file
let read_req = VfsRequest::Read { fd, len: 1024, offset: 0 }; // Read up to 1024 bytes from offset 0
match vfs_chan.send_and_recv::<VfsRequest, VfsResponse>(&read_req) {
    Ok(VfsResponse::Data(data)) => {
        let content = String::from_utf8_lossy(&data);
        log!("Read {} bytes from file: \n{}", data.len(), content);
    },
    Ok(VfsResponse::Error { code, message }) => {
        log!("Failed to read from file: {} ({})", message, code);
    },
    _ => { log!("Unexpected response during file read."); }
};

// 3. Close the file
let close_req = VfsRequest::Close { fd };
match vfs_chan.send_and_recv::<VfsRequest, VfsResponse>(&close_req) {
    Ok(VfsResponse::Success(_)) => {
        log!("File closed successfully.");
    },
    _ => { log!("Failed to close file or unexpected response."); }
};
```

### Example 2: Listing a Directory

```rust
// Pseudocode for client V-Node

let mut vfs_chan = VNodeChannel::new(7);

let list_req = VfsRequest::List { path: String::from("/") }; // List root directory
match vfs_chan.send_and_recv::<VfsRequest, VfsResponse>(&list_req) {
    Ok(VfsResponse::DirectoryEntries(entries)) => {
        log!("Directory entries for /:");
        for (name, metadata) in entries {
            log!("- {}: {} ({} bytes)", name, if metadata.is_dir { "Directory" } else { "File" }, metadata.size);
        }
    },
    Ok(VfsResponse::Error { code, message }) => {
        log!("Failed to list directory: {} ({})", message, code);
    },
    _ => { log!("Unexpected response during directory listing."); }
};
```

This documentation outlines the critical role of the VFS in providing a unified and secure file system interface in AetherOS, demonstrating its modularity and reliance on IPC for system interaction.
