
# File Manager V-Node (svc://file-manager)

## Overview

The `file-manager` V-Node provides a high-level API for managing files and directories within AetherOS. It acts as a user-facing application or a backend for graphical file explorers, abstracting away the complexities of direct VFS interactions. This V-Node enables operations such as browsing, copying, moving, deleting, and creating files and directories, all while leveraging the underlying `svc://vfs` V-Node for actual filesystem access.

## IPC Protocol

Communication with the `file-manager` V-Node occurs via IPC, using the `FileManagerRequest` and `FileManagerResponse` enums defined in `src/ipc/file_manager_ipc.rs`.

### FileManagerRequest Enum (Client -> file-manager)

Client V-Nodes (e.g., a GUI file explorer or another application) send these requests to `svc://file-manager` to perform file management operations.

```rust
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
```

**Parameters:**

*   `path`: A `String` representing the absolute path to the directory to browse, or the target path for deletion/creation.
*   `source`: A `String` representing the absolute path of the source file or directory for copy/move operations.
*   `destination`: A `String` representing the absolute path of the destination for copy/move operations.

### FileManagerResponse Enum (file-manager -> Client)

`svc://file-manager` sends these responses back to the client V-Node after processing a `FileManagerRequest`.

```rust
#[derive(Debug, Serialize, Deserialize)]
pub enum FileManagerResponse {
    /// Indicates a successful operation, with an optional descriptive message.
    Success(String),
    /// Indicates an error occurred during the operation.
    Error(String),
    /// Returns a list of directory entries (name, metadata).
    DirectoryEntries(BTreeMap<String, VfsMetadata>),
}
```

**Return Values:**

*   `Success(String)`: A successful operation, with an optional descriptive message (e.g., "File copied successfully").
*   `Error(String)`: An error occurred during the operation, with a descriptive message.
*   `DirectoryEntries(BTreeMap<String, VfsMetadata>)`: Returns a map of directory entry names to their `VfsMetadata` when a `Browse` request is successful.

## Functionality

The `file-manager` V-Node performs the following key functions:

1.  **IPC Interface**: Exposes a well-defined IPC interface for client applications to request file management actions.
2.  **VFS Interaction**: Delegates all actual file system operations (listing, reading, writing, moving, deleting) to the `svc://vfs` V-Node via IPC.
3.  **High-Level Operations**: Orchestrates multi-step operations like `Copy` and `Move` by issuing sequences of `Read`, `Write`, `Open`, `Close`, and potentially `Delete` requests to `svc://vfs`.
4.  **Error Translation**: Translates specific VFS errors into more general `FileManagerResponse::Error` messages.
5.  **User Context**: (Conceptual) May eventually interact with user identity (AID) to enforce permissions or personalize file views.

## Usage Examples

### Example 1: Browsing a Directory

```rust
// Pseudocode for client V-Node (e.g., a GUI file explorer) wanting to browse a directory

let mut file_manager_chan = VNodeChannel::new(9); // IPC Channel to svc://file-manager

let request = FileManagerRequest::Browse { path: String::from("/home/user/documents") };
match file_manager_chan.send_and_recv::<FileManagerRequest, FileManagerResponse>(&request) {
    Ok(FileManagerResponse::DirectoryEntries(entries)) => {
        log!("Contents of /home/user/documents:");
        for (name, metadata) in entries {
            log!("- {}: {} ({} bytes)", name, if metadata.is_dir { "Dir" } else { "File" }, metadata.size);
        }
    },
    Ok(FileManagerResponse::Error(msg)) => {
        log!("Failed to browse directory: {}", msg);
    },
    _ => log!("Unexpected response from File Manager"),
}
```

### Example 2: Copying a File

```rust
// Pseudocode for client V-Node wanting to copy a file

let mut file_manager_chan = VNodeChannel::new(9);

let request = FileManagerRequest::Copy {
    source: String::from("/home/user/document.txt"),
    destination: String::from("/home/user/backups/document.txt"),
};
match file_manager_chan.send_and_recv::<FileManagerRequest, FileManagerResponse>(&request) {
    Ok(FileManagerResponse::Success(msg)) => {
        log!("File copy successful: {}", msg);
    },
    Ok(FileManagerResponse::Error(msg)) => {
        log!("File copy failed: {}", msg);
    },
    _ => log!("Unexpected response from File Manager"),
}
```

This documentation highlights the `file-manager` V-Node's role as a key application-level service, simplifying file operations for users and other V-Nodes by abstracting the VFS layer and adhering to AetherOS's IPC and security principles.
