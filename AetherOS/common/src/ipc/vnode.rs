// common/src/ipc/vnode.rs

#![no_std]

extern crate alloc;

use alloc::vec::Vec;
use crate::ipc::{IpcSend, IpcRecv};
use crate::syscall::{syscall3, SYS_IPC_SEND, SYS_IPC_RECV, SYS_IPC_RECV_NONBLOCKING, SUCCESS, E_ERROR};

pub struct VNodeChannel {
    pub id: u32,
    buffer: [u8; 4096],
}

impl VNodeChannel {
    pub fn new(id: u32) -> Self {
        Self { id, buffer: [0; 4096] }
    }

    pub fn recv_blocking(&mut self) -> Result<Vec<u8>, ()> {
        loop {
            let len = unsafe {
                syscall3(
                    SYS_IPC_RECV,
                    self.id as u64,
                    self.buffer.as_mut_ptr() as u64,
                    self.buffer.len() as u64 // Pass max capacity
                )
            };
            match len {
                l if l > SUCCESS => { // Message received, 'l' is the length
                    return Ok(self.buffer[..l as usize].to_vec());
                },
                SUCCESS => { // SUCCESS (0) means kernel blocked us or no message yet if non-blocking
                    // In the blocking syscall, if 0 is returned, it means the kernel
                    // successfully blocked the task and will re-schedule it later.
                    // So we just continue the loop when re-scheduled to try receiving again.
                },
                E_ERROR => { // Error from syscall
                    return Err(());
                },
                _ => { // Other error codes or unexpected values
                    return Err(());
                }
            }
        }
    }

    pub fn recv_non_blocking(&mut self) -> Result<Option<Vec<u8>>, ()> {
        let len = unsafe {
            syscall3(
                SYS_IPC_RECV_NONBLOCKING,
                self.id as u64,
                self.buffer.as_mut_ptr() as u64,
                self.buffer.len() as u64 // Pass max capacity
            )
        };
        match len {
            l if l > SUCCESS => { // Message received
                Ok(Some(self.buffer[..l as usize].to_vec()))
            },
            SUCCESS => { // No message available, but no error
                Ok(None)
            },
            E_ERROR => { // Error from syscall
                Err(())
            },
            _ => Err(())
        }
    }

    pub fn send_and_recv<Req: serde::Serialize, Resp: serde::de::DeserializeOwned>(
        &mut self, request: &Req
    ) -> Result<Resp, ()> {
        let serialized_request = postcard::to_allocvec(request).map_err(|_| ())?;
        self.send_raw(&serialized_request)?;
        
        // After sending, immediately try to receive the response.
        // This assumes a synchronous request-response pattern.
        match self.recv_blocking() {
            Ok(data) => postcard::from_bytes(&data).map_err(|_| ())?,
            Err(_) => Err(()),
        }
    }
}

impl IpcSend for VNodeChannel {
    fn send_raw(&mut self, bytes: &[u8]) -> Result<(), ()> {
        unsafe {
            let res = syscall3(
                SYS_IPC_SEND,
                self.id as u64,
                bytes.as_ptr() as u64,
                bytes.len() as u64,
            );
            if res == SUCCESS { Ok(()) } else { Err(()) }
        }
    }

    fn send<T: serde::Serialize>(&mut self, msg: &T) -> Result<(), ()> {
        let serialized = postcard::to_allocvec(msg).map_err(|_| ())?;
        self.send_raw(&serialized)
    }
}

impl IpcRecv for VNodeChannel {
    fn recv<T: serde::de::DeserializeOwned>(&mut self) -> Option<T> {
        match self.recv_non_blocking() {
            Ok(Some(data)) => postcard::from_bytes(&data).ok(),
            _ => None,
        }
    }
}
