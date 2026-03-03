#![no_std]
#![no_main]

extern crate alloc;

use core::panic::PanicInfo;
use alloc::vec::Vec;
use alloc::collections::BTreeMap;
use alloc::format;
use alloc::string::{String, ToString};

use crate::ipc::vnode::VNodeChannel;
use crate::syscall::{syscall3, SYS_LOG, SUCCESS, SYS_TIME};
use crate::ipc::vfs_ipc::{VfsRequest, VfsResponse, Fd, VfsMetadata};

// Temporary log function for V-Nodes
fn log(msg: &str) {
    unsafe {
        let res = syscall3(
            SYS_LOG,
            msg.as_ptr() as u64,
            msg.len() as u64,
            0 // arg3 is unused for SYS_LOG
        );
        if res != SUCCESS { /* Handle log error, maybe panic or fall back */ }
    }
}

// Placeholder for an open file handle in the VFS
#[derive(Debug)]
struct OpenFile {
    path: String,
    flags: u32,
    cursor: u64,
    // Conceptual: backend-specific handle (e.g., AetherFS handle, Ramdisk handle)
    backend_handle: u64, // Dummy handle for backend communication
}

struct VfsService {
    client_chan: VNodeChannel,
    aetherfs_chan: VNodeChannel, // Channel to AetherFS backend
    // ramdisk_chan: VNodeChannel, // Conceptual: Channel to RAM disk backend
    // disk_driver_chan: VNodeChannel, // Conceptual: Channel to block device backend

    next_fd: Fd,
    open_files: BTreeMap<Fd, OpenFile>,
}

impl VfsService {
    fn new(client_chan_id: u32, aetherfs_chan_id: u32) -> Self {
        let client_chan = VNodeChannel::new(client_chan_id);
        let aetherfs_chan = VNodeChannel::new(aetherfs_chan_id);

        log("VFS Service: Initializing...");

        Self {
            client_chan,
            aetherfs_chan,
            next_fd: 1,
            open_files: BTreeMap::new(),
        }
    }

    fn handle_request(&mut self, request: VfsRequest) -> VfsResponse {
        match request {
            VfsRequest::Open { path, flags } => {
                log(&alloc::format!("VFS: Open request for path: {} with flags: {}.", path, flags));
                // Conceptual: Send IPC to AetherFS or other backend to open/create file
                // For now, simulate success and create a dummy OpenFile entry.
                // In a real scenario, the backend would return its own handle.
                let backend_handle = 1000 + self.next_fd as u64; // Dummy backend handle

                let fd = self.next_fd;
                self.next_fd += 1;
                self.open_files.insert(fd, OpenFile { path: path.clone(), flags, cursor: 0, backend_handle });
                log(&alloc::format!("VFS: Opened {} as fd {}.", path, fd));
                VfsResponse::Success(fd as i32)
            },
            VfsRequest::Read { fd, len, offset } => {
                if let Some(file) = self.open_files.get_mut(&fd) {
                    log(&alloc::format!("VFS: Read request for fd: {}, len: {}, offset: {}.", fd, len, offset));
                    // Conceptual: Send IPC to backend (e.g., AetherFS) to read data
                    // For now, return dummy data and simulate backend read.
                    // The actual `read` operation would involve sending a request to `aetherfs_chan`
                    // with file.backend_handle, offset, and len.

                    // Simulate reading from AetherFS backend
                    // Example: `self.aetherfs_chan.send_and_recv(&AetherFsRequest::Read { handle: file.backend_handle, offset, len })`
                    let dummy_data = alloc::format!("dummy_data_from_file_{}_at_offset_{}", file.path, offset).as_bytes().to_vec();

                    let bytes_to_read = len.min(dummy_data.len() as u32) as usize;
                    let mut response_data = Vec::with_capacity(bytes_to_read);
                    response_data.extend_from_slice(&dummy_data[..bytes_to_read]);

                    file.cursor = offset + response_data.len() as u64;
                    log(&alloc::format!("VFS: Read {} bytes from fd {} at offset {}.", response_data.len(), fd, offset));
                    VfsResponse::Data(response_data)
                } else {
                    log(&alloc::format!("VFS: Read failed, bad fd: {}.", fd));
                    VfsResponse::Error { code: 9, message: "Bad file descriptor".to_string() } // EBADF
                }
            },
            VfsRequest::Write { fd, data, offset } => {
                if let Some(file) = self.open_files.get_mut(&fd) {
                    log(&alloc::format!("VFS: Write request for fd: {}, len: {}, offset: {}.", fd, data.len(), offset));
                    // Conceptual: Send IPC to backend (e.g., AetherFS) to write data
                    // The actual `write` operation would involve sending a request to `aetherfs_chan`
                    // with file.backend_handle, offset, and data.

                    // Simulate writing to AetherFS backend
                    // Example: `self.aetherfs_chan.send_and_recv(&AetherFsRequest::Write { handle: file.backend_handle, offset, data })`

                    file.cursor = offset + data.len() as u64;
                    log(&alloc::format!("VFS: Wrote {} bytes to fd {} at offset {}.", data.len(), fd, offset));
                    VfsResponse::Success(data.len() as i32)
                } else {
                    log(&alloc::format!("VFS: Write failed, bad fd: {}.", fd));
                    VfsResponse::Error { code: 9, message: "Bad file descriptor".to_string() } // EBADF
                }
            },
            VfsRequest::List { path } => {
                log(&alloc::format!("VFS: List request for path: {}.", path));
                // Conceptual: Send IPC to backend to list directory contents
                // Example: `self.aetherfs_chan.send_and_recv(&AetherFsRequest::ListDir { path: path.clone() })`
                let mut entries = BTreeMap::new();
                if path == "/" {
                    entries.insert("home".to_string(), VfsMetadata { is_dir: true, size: 0, created: 0, modified: 0, permissions: 0o755 });
                    entries.insert("etc".to_string(), VfsMetadata { is_dir: true, size: 0, created: 0, modified: 0, permissions: 0o755 });
                    entries.insert("bin".to_string(), VfsMetadata { is_dir: true, size: 0, created: 0, modified: 0, permissions: 0o755 });
                    entries.insert("README.txt".to_string(), VfsMetadata { is_dir: false, size: 1024, created: 0, modified: 0, permissions: 0o644 });
                } else if path == "/home" {
                    entries.insert("user".to_string(), VfsMetadata { is_dir: true, size: 0, created: 0, modified: 0, permissions: 0o755 });
                } else if path == "/home/user" {
                    entries.insert("documents".to_string(), VfsMetadata { is_dir: true, size: 0, created: 0, modified: 0, permissions: 0o755 });
                    entries.insert("config.txt".to_string(), VfsMetadata { is_dir: false, size: 256, created: 0, modified: 0, permissions: 0o644 });
                } else {
                    return VfsResponse::Error { code: 2, message: format!("Path not found: {}", path) }; // ENOENT
                }
                log(&alloc::format!("VFS: Listed {} entries for path {}.", entries.len(), path));
                VfsResponse::DirectoryEntries(entries)
            },
            VfsRequest::Stat { path } => {
                log(&alloc::format!("VFS: Stat request for path: {}.", path));
                // Conceptual: Send IPC to backend to get metadata
                // Example: `self.aetherfs_chan.send_and_recv(&AetherFsRequest::Stat { path: path.clone() })`
                if path == "/README.txt" {
                    log(&alloc::format!("VFS: Returned metadata for {}.", path));
                    VfsResponse::Metadata(VfsMetadata { is_dir: false, size: 1024, created: 1678886400, modified: 1678886400, permissions: 0o644 })
                } else if path == "/home" {
                    log(&alloc::format!("VFS: Returned metadata for {}.", path));
                    VfsResponse::Metadata(VfsMetadata { is_dir: true, size: 0, created: 1678886400, modified: 1678886400, permissions: 0o755 })
                } else {
                    log(&alloc::format!("VFS: Path not found for stat: {}.", path));
                    VfsResponse::Error { code: 2, message: format!("Path not found: {}", path) } // ENOENT
                }
            },
            VfsRequest::Close { fd } => {
                if let Some(file) = self.open_files.remove(&fd) {
                    log(&alloc::format!("VFS: Closed fd {} (path: {}).", fd, file.path));
                    // Conceptual: Send IPC to backend to close file handle
                    // Example: `self.aetherfs_chan.send_and_recv(&AetherFsRequest::Close { handle: file.backend_handle })`
                    VfsResponse::Success(0)
                } else {
                    log(&alloc::format!("VFS: Close failed, bad fd: {}.", fd));
                    VfsResponse::Error { code: 9, message: "Bad file descriptor".to_string() } // EBADF
                }
            },
            VfsRequest::Delete { path } => {
                log(&alloc::format!("VFS: Delete request for path: {}.", path));
                // Conceptual: Send IPC to backend to delete file/directory.
                // For now, simulate success.
                VfsResponse::DeleteSuccess
            },
            VfsRequest::CreateDirectory { path } => {
                log(&alloc::format!("VFS: Create directory request for path: {}.", path));
                // Conceptual: Send IPC to backend to create directory.
                // For now, simulate success.
                VfsResponse::CreateDirectorySuccess
            },
            VfsRequest::Move { source, destination } => {
                log(&alloc::format!("VFS: Move request from {} to {}.", source, destination));
                // Conceptual: Send IPC to backend to move/rename file/directory.
                // For now, simulate success.
                VfsResponse::MoveSuccess
            },
        }
    }

    fn run_loop(&mut self) -> ! {
        log("VFS Service: Entering main event loop.");
        loop {
            // Process incoming requests from client V-Nodes
            if let Ok(Some(req_data)) = self.client_chan.recv_non_blocking() {
                if let Ok(request) = postcard::from_bytes::<VfsRequest>(&req_data) {
                    log(&alloc::format!("VFS Service: Received VfsRequest: {:?}.", request));
                    let response = self.handle_request(request);
                    self.client_chan.send(&response).unwrap_or_else(|_| log("VFS Service: Failed to send response to client."));
                } else {
                    log("VFS Service: Failed to deserialize VfsRequest from client.");
                }
            }

            // Yield to other V-Nodes to prevent busy-waiting
            unsafe { syscall3(SYS_TIME, 0, 0, 0); } // This will cause a context switch
        }
    }
}

#[no_mangle]
pub extern "C" fn _start() -> ! {
    // Assuming channel ID 7 for VFS Service for client requests
    // Assuming channel ID 6 for AetherFS backend (conceptual)
    let mut vfs_service = VfsService::new(7, 6);
    vfs_service.run_loop();
}

#[panic_handler]
pub extern "C" fn panic(info: &PanicInfo) -> ! {
    log(&alloc::format!("VFS V-Node panicked! Info: {:?}.", info));
    // In a production system, this might trigger a system-wide error handler or reboot.
    // For now, it enters an infinite loop to prevent further execution.
    loop {}
}
