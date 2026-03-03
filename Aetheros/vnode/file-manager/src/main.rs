// vnode/file-manager/src/main.rs

#![no_std]
#![no_main]

extern crate alloc;

use core::panic::PanicInfo;
use alloc::vec::Vec;
use alloc::collections::BTreeMap;
use alloc::format;
use alloc::string::{String, ToString};

use common::ipc::vnode::VNodeChannel;
use common::syscall::{syscall3, SYS_LOG, SUCCESS, SYS_TIME};
use common::ipc::file_manager_ipc::{FileManagerRequest, FileManagerResponse};
use common::ipc::vfs_ipc::{VfsRequest, VfsResponse, Fd, VfsMetadata};

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

struct FileManagerService {
    client_chan: VNodeChannel, // Channel for AetherTerminal or other client V-Nodes
    vfs_chan: VNodeChannel, // Channel to svc://vfs
}

impl FileManagerService {
    fn new(client_chan_id: u32, vfs_chan_id: u32) -> Self {
        let client_chan = VNodeChannel::new(client_chan_id);
        let vfs_chan = VNodeChannel::new(vfs_chan_id);

        log("File Manager Service: Initializing...");

        Self {
            client_chan,
            vfs_chan,
        }
    }

    fn handle_request(&mut self, request: FileManagerRequest) -> FileManagerResponse {
        match request {
            FileManagerRequest::Browse { path } => {
                log(&alloc::format!("File Manager: Browse request for path: {}.", path));
                match self.vfs_chan.send_and_recv::<VfsRequest, VfsResponse>(&VfsRequest::List { path: path.clone() }) {
                    Ok(VfsResponse::DirectoryEntries(entries)) => {
                        log(&alloc::format!("File Manager: Successfully browsed {}. Found {} entries.", path, entries.len()));
                        FileManagerResponse::DirectoryEntries(entries)
                    },
                    Ok(VfsResponse::Error { message, .. }) => {
                        log(&alloc::format!("File Manager: Failed to browse {}: {}.", path, message));
                        FileManagerResponse::Error(format!("Failed to browse {}: {}", path, message))
                    },
                    _ => {
                        log("File Manager: Unexpected response from VFS during browse.");
                        FileManagerResponse::Error("Unexpected response from VFS during browse".to_string())
                    },
                }
            },
            FileManagerRequest::Copy { source, destination } => {
                log(&alloc::format!("File Manager: Copy request from {} to {}.", source, destination));

                // Step 1: Open source file for reading
                let src_fd = match self.vfs_chan.send_and_recv::<VfsRequest, VfsResponse>(&VfsRequest::Open { path: source.clone(), flags: 0 /* O_RDONLY */ }) {
                    Ok(VfsResponse::Success(fd)) => fd as Fd,
                    Ok(VfsResponse::Error { message, .. }) => return FileManagerResponse::Error(format!("Failed to open source file {}: {}", source, message)),
                    _ => return FileManagerResponse::Error("Unexpected VFS response opening source file".to_string()),
                };

                // Step 2: Open destination file for writing (create if not exists, truncate if exists)
                let dest_fd = match self.vfs_chan.send_and_recv::<VfsRequest, VfsResponse>(&VfsRequest::Open { path: destination.clone(), flags: 1 /* O_WRONLY | O_CREAT | O_TRUNC */ }) {
                    Ok(VfsResponse::Success(fd)) => fd as Fd,
                    Ok(VfsResponse::Error { message, .. }) => {
                        // Close source file before returning error
                        let _ = self.vfs_chan.send_and_recv::<VfsRequest, VfsResponse>(&VfsRequest::Close { fd: src_fd });
                        return FileManagerResponse::Error(format!("Failed to open/create destination file {}: {}", destination, message));
                    },
                    _ => {
                        let _ = self.vfs_chan.send_and_recv::<VfsRequest, VfsResponse>(&VfsRequest::Close { fd: src_fd });
                        return FileManagerResponse::Error("Unexpected VFS response opening destination file".to_string());
                    },
                };

                let mut offset = 0;
                let mut bytes_copied = 0;
                const CHUNK_SIZE: u32 = 4096; // Read/write in 4KB chunks

                loop {
                    // Read a chunk from source
                    let read_resp = self.vfs_chan.send_and_recv::<VfsRequest, VfsResponse>(&VfsRequest::Read { fd: src_fd, len: CHUNK_SIZE, offset });
                    let data = match read_resp {
                        Ok(VfsResponse::Data(d)) => d,
                        Ok(VfsResponse::Error { message, .. }) => {
                            let _ = self.vfs_chan.send_and_recv::<VfsRequest, VfsResponse>(&VfsRequest::Close { fd: src_fd });
                            let _ = self.vfs_chan.send_and_recv::<VfsRequest, VfsResponse>(&VfsRequest::Close { fd: dest_fd });
                            return FileManagerResponse::Error(format!("Error reading from source {}: {}", source, message));
                        },
                        _ => {
                            let _ = self.vfs_chan.send_and_recv::<VfsRequest, VfsResponse>(&VfsRequest::Close { fd: src_fd });
                            let _ = self.vfs_chan.send_and_recv::<VfsRequest, VfsResponse>(&VfsRequest::Close { fd: dest_fd });
                            return FileManagerResponse::Error("Unexpected VFS response reading source file".to_string());
                        },
                    };

                    if data.is_empty() {
                        // End of file
                        break;
                    }

                    // Write the chunk to destination
                    let write_resp = self.vfs_chan.send_and_recv::<VfsRequest, VfsResponse>(&VfsRequest::Write { fd: dest_fd, data: data.clone(), offset });
                    match write_resp {
                        Ok(VfsResponse::Success(bytes_written)) if bytes_written as usize == data.len() => {
                            offset += data.len() as u64;
                            bytes_copied += data.len();
                        },
                        Ok(VfsResponse::Error { message, .. }) => {
                            let _ = self.vfs_chan.send_and_recv::<VfsRequest, VfsResponse>(&VfsRequest::Close { fd: src_fd });
                            let _ = self.vfs_chan.send_and_recv::<VfsRequest, VfsResponse>(&VfsRequest::Close { fd: dest_fd });
                            return FileManagerResponse::Error(format!("Error writing to destination {}: {}", destination, message));
                        },
                        _ => {
                            let _ = self.vfs_chan.send_and_recv::<VfsRequest, VfsResponse>(&VfsRequest::Close { fd: src_fd });
                            let _ = self.vfs_chan.send_and_recv::<VfsRequest, VfsResponse>(&VfsRequest::Close { fd: dest_fd });
                            return FileManagerResponse::Error("Unexpected VFS response writing destination file".to_string());
                        },
                    };
                }

                // Close both files
                let _ = self.vfs_chan.send_and_recv::<VfsRequest, VfsResponse>(&VfsRequest::Close { fd: src_fd });
                let _ = self.vfs_chan.send_and_recv::<VfsRequest, VfsResponse>(&VfsRequest::Close { fd: dest_fd });

                log(&alloc::format!("File Manager: Successfully copied {} bytes from {} to {}.", bytes_copied, source, destination));
                FileManagerResponse::Success(format!("Successfully copied {} to {} ({} bytes)", source, destination, bytes_copied))
            },
            FileManagerRequest::Move { source, destination } => {
                log(&alloc::format!("File Manager: Move request from {} to {}.", source, destination));
                match self.vfs_chan.send_and_recv::<VfsRequest, VfsResponse>(&VfsRequest::Move { source: source.clone(), destination: destination.clone() }) {
                    Ok(VfsResponse::MoveSuccess) => {
                        log(&alloc::format!("File Manager: Successfully moved {} to {}.", source, destination));
                        FileManagerResponse::Success(format!("Successfully moved {} to {}", source, destination))
                    },
                    Ok(VfsResponse::Error { message, .. }) => {
                        log(&alloc::format!("File Manager: Failed to move {} to {}: {}.", source, destination, message));
                        FileManagerResponse::Error(format!("Failed to move {} to {}: {}", source, destination, message))
                    },
                    _ => {
                        log("File Manager: Unexpected response from VFS during move.");
                        FileManagerResponse::Error("Unexpected response from VFS during move".to_string())
                    },
                }
            },
            FileManagerRequest::Delete { path } => {
                log(&alloc::format!("File Manager: Delete request for path: {}.", path));
                match self.vfs_chan.send_and_recv::<VfsRequest, VfsResponse>(&VfsRequest::Delete { path: path.clone() }) {
                    Ok(VfsResponse::DeleteSuccess) => {
                        log(&alloc::format!("File Manager: Successfully deleted {}.", path));
                        FileManagerResponse::Success(format!("Successfully deleted {}", path))
                    },
                    Ok(VfsResponse::Error { message, .. }) => {
                        log(&alloc::format!("File Manager: Failed to delete {}: {}.", path, message));
                        FileManagerResponse::Error(format!("Failed to delete {}: {}", path, message))
                    },
                    _ => {
                        log("File Manager: Unexpected response from VFS during delete.");
                        FileManagerResponse::Error("Unexpected response from VFS during delete".to_string())
                    },
                }
            },
            FileManagerRequest::CreateDirectory { path } => {
                log(&alloc::format!("File Manager: Create directory request for path: {}.", path));
                match self.vfs_chan.send_and_recv::<VfsRequest, VfsResponse>(&VfsRequest::CreateDirectory { path: path.clone() }) {
                    Ok(VfsResponse::CreateDirectorySuccess) => {
                        log(&alloc::format!("File Manager: Successfully created directory {}.", path));
                        FileManagerResponse::Success(format!("Successfully created directory {}", path))
                    },
                    Ok(VfsResponse::Error { message, .. }) => {
                        log(&alloc::format!("File Manager: Failed to create directory {}: {}.", path, message));
                        FileManagerResponse::Error(format!("Failed to create directory {}: {}", path, message))
                    },
                    _ => {
                        log("File Manager: Unexpected response from VFS during create directory.");
                        FileManagerResponse::Error("Unexpected response from VFS during create directory".to_string())
                    },
                }
            },
        }
    }

    fn run_loop(&mut self) -> ! {
        log("File Manager Service: Entering main event loop.");
        loop {
            // Process incoming requests from client V-Nodes
            if let Ok(Some(req_data)) = self.client_chan.recv_non_blocking() {
                if let Ok(request) = postcard::from_bytes::<FileManagerRequest>(&req_data) {
                    log(&alloc::format!("File Manager Service: Received FileManagerRequest: {:?}.", request));
                    let response = self.handle_request(request);
                    self.client_chan.send(&response).unwrap_or_else(|_| log("File Manager Service: Failed to send response to client."));
                } else {
                    log("File Manager Service: Failed to deserialize FileManagerRequest.");
                }
            }

            // Yield to other V-Nodes to prevent busy-waiting
            unsafe { syscall3(SYS_TIME, 0, 0, 0); } // This will cause a context switch
        }
    }
}

#[no_mangle]
pub extern "C" fn _start() -> ! {
    // Assuming channel IDs:
    // 9 for File Manager Service client requests
    // 7 for VFS Service
    let mut file_manager_service = FileManagerService::new(9, 7);
    file_manager_service.run_loop();
}

#[panic_handler]
pub extern "C" fn panic(info: &PanicInfo) -> ! {
    log(&alloc::format!("File Manager V-Node panicked! Info: {:?}.", info));
    loop {}
}