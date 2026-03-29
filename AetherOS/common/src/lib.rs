#![no_std]

#[cfg(feature = "alloc")]
extern crate alloc;

#[cfg(feature = "serde")]
extern crate serde;

pub mod ipc;
pub mod syscall;
pub mod swarm_engine;
pub mod nexus_net_transport;
pub mod arp_dht;
pub mod trust;
pub mod examples;

pub mod channel;
pub mod message;
pub mod nexus_msg;

pub mod ui;

pub use ipc::*;
pub use ui::*;
