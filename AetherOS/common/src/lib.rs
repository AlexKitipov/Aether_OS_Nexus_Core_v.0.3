#![no_std]

extern crate alloc;

pub mod cid;
pub mod manifest;
pub mod trust;
pub mod arp_dht;
pub mod swarm_engine;
pub mod ipc;
pub mod syscall;

// Temporarily include kernel and vnode modules for cross-crate access during development
// In a final structure, V-Nodes would communicate via IPC, not direct module imports.
pub mod kernel;
pub mod vnode;

pub mod socket_ipc;
pub mod dns_ipc;
pub mod init_ipc;
pub mod vfs_ipc;
pub mod shell_ipc;
pub mod file_manager_ipc;
pub mod mail_ipc;
pub mod model_runtime_ipc;

// Explicitly declare and re-export nexus_net_transport module
pub mod nexus_net_transport;
pub use nexus_net_transport::*;

pub mod ui_protocol;
pub use ui_protocol::*;

pub mod ui;
pub use ui::*;
