# File Manager V-Node

## Overview

The `file-manager` V-Node provides a high-level interface for managing user files and directories within AetherOS. It acts as a client to the `vfs` V-Node, translating user-friendly file operations (like copy, move, delete, browse) into lower-level Virtual File System requests.

## Core Responsibilities

*   **File and Directory Operations**: Exposes IPC endpoints for clients to perform common file management tasks such as browsing directory contents, copying files/directories, moving/renaming, deleting, and creating new directories.
*   **VFS Interaction**: Communicates with the `vfs` V-Node to execute the actual filesystem operations. It handles the details of opening, reading, writing, and closing files, as well as listing and stat-ing directory entries.
*   **Error Handling**: Translates errors from the `vfs` V-Node into more descriptive `FileManagerResponse` error messages.
*   **Resource Management**: Manages temporary resources (e.g., during file copy operations) by utilizing its designated `/tmp` volatile storage.

## Capabilities and Dependencies

To perform its functions, the `file-manager` V-Node requires specific capabilities:

*   `CAP_IPC_ACCEPT`: To accept file management requests from client V-Nodes (e.g., a graphical file explorer, `AetherShell`).
*   `CAP_IPC_CONNECT: "svc://vfs"`: To interact with the underlying Virtual File System for all file and directory operations.
*   `CAP_LOG_WRITE`: For logging file operation events, progress, and errors.
*   `CAP_TIME_READ`: For potential timestamping operations or for cache invalidation within its own logic.

## Operational Flow (High-Level)

1.  **Initialization**:
    *   Establishes its IPC channels with clients and the `vfs` V-Node.
2.  **Request Handling**:
    *   Receives `FileManagerRequest` messages (e.g., `Browse`, `Copy`, `Move`, `Delete`, `CreateDirectory`) from client V-Nodes.
    *   For `Browse` requests, it sends a `VfsRequest::List` to `vfs` and returns the `DirectoryEntries`.
    *   For `Copy` requests, it involves multiple `VfsRequest::Open`, `VfsRequest::Read`, `VfsRequest::Write`, and `VfsRequest::Close` calls to stream data from source to destination.
    *   For `Move`, `Delete`, and `CreateDirectory` requests, it forwards the corresponding `VfsRequest` to `vfs`.
    *   Processes responses from `vfs` and formats them into `FileManagerResponse` messages (Success or Error).
3.  **Event Loop**: Continuously polls its client IPC channel for new requests and processes them. Uses `SYS_TIME` to yield control to the kernel, allowing other V-Nodes to run.

## Example `vnode.yml` Configuration

```yaml
# vnode/file-manager/vnode.yml
vnode:
  name: "file-manager"
  version: "0.1.0"
  maintainer: "aetheros-core-team@aetheros.org"
  mode: strict # A core system component for managing user files

runtime:
  entrypoint: "bin/file-manager.vnode"
  required_mem_mb: 32 # For file buffering, directory listings, and IPC buffers
  max_cpu_share: 0.10 # Can be bursty depending on file operations (copy/move)

capabilities:
  - CAP_IPC_ACCEPT # To accept requests from client V-Nodes (e.g., GUI file explorer)
  - CAP_IPC_CONNECT: "svc://vfs" # To interact with the VFS for all file operations
  - CAP_LOG_WRITE # For logging file operations and errors
  - CAP_TIME_READ # For timestamping operations or for cache invalidation

storage:
  mounts:
    - path: "/home/<AID>"
      source: "aetherfs://user/<AID>"
      options: [ "rw", "recursive" ] # Full read/write access to user's home directory
    - path: "/tmp"
      source: "volatile://ramdisk"
      size: "16MB" # For temporary files during copy/move operations

observability:
  metrics: ["files_copied_total", "files_moved_total", "files_deleted_total", "dirs_created_total", "browse_requests_total", "errors_total"]
```
