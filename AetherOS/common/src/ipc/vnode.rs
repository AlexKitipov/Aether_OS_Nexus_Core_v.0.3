// common/src/ipc/vnode.rs

extern crate alloc;

use alloc::vec::Vec;
use crate::ipc::{IpcSend, IpcRecv};
use crate::syscall::{
    syscall3, E_ACC_DENIED, E_ERROR, SUCCESS, SYS_IPC_RECV, SYS_IPC_RECV_NONBLOCKING, SYS_IPC_SEND, SYS_TIME,
};

const DEFAULT_IPC_BUFFER_CAPACITY: usize = 8192;

#[cfg(feature = "serde")]
const MAX_SERIALIZED_REQUEST_SIZE: usize = 4096;

pub struct VNodeChannel {
    pub id: u32,
    buffer: Vec<u8>,
}

impl VNodeChannel {
    pub fn new(id: u32) -> Self {
        Self { id, buffer: Vec::new() }
    }

    fn ensure_buffer_capacity(&mut self, min_capacity: usize) {
        if self.buffer.len() < min_capacity {
            self.buffer.resize(min_capacity, 0);
        }
    }

    pub fn recv_blocking_bytes(&mut self) -> core::result::Result<&[u8], ()> {
        self.ensure_buffer_capacity(DEFAULT_IPC_BUFFER_CAPACITY);
        let mut backoff = 1;

        loop {
            let len = syscall3(
                SYS_IPC_RECV,
                self.id as u64,
                self.buffer.as_mut_ptr() as u64,
                self.buffer.len() as u64,
            );
            match len {
                SUCCESS => {
                    for _ in 0..backoff {
                        core::hint::spin_loop();
                    }
                    if backoff < 128 {
                        backoff <<= 1;
                    }
                    let _ = syscall3(SYS_TIME, 0, 0, 0);
                }
                E_ERROR | E_ACC_DENIED => return Err(()),
                l => {
                    let msg_len = l as usize;
                    if msg_len > self.buffer.len() {
                        return Err(());
                    }
                    return Ok(&self.buffer[..msg_len]);
                }
            }
        }
    }

    pub fn recv_blocking(&mut self) -> core::result::Result<Vec<u8>, ()> {
        self.recv_blocking_bytes().map(|slice| slice.to_vec())
    }

    pub fn recv_non_blocking(&mut self) -> core::result::Result<Option<Vec<u8>>, ()> {
        self.ensure_buffer_capacity(DEFAULT_IPC_BUFFER_CAPACITY);

        let len = syscall3(
            SYS_IPC_RECV_NONBLOCKING,
            self.id as u64,
            self.buffer.as_mut_ptr() as u64,
            self.buffer.len() as u64,
        );
        match len {
            SUCCESS => Ok(None),
            E_ERROR | E_ACC_DENIED => Err(()),
            l => {
                let msg_len = l as usize;
                if msg_len > self.buffer.len() {
                    return Err(());
                }
                Ok(Some(self.buffer[..msg_len].to_vec()))
            }
        }
    }

    #[cfg(feature = "serde")]
    pub fn send_and_recv<Req: serde::Serialize, Resp: serde::de::DeserializeOwned>(
        &mut self, request: &Req
    ) -> core::result::Result<Resp, ()> {
        let mut serialized_request = [0u8; MAX_SERIALIZED_REQUEST_SIZE];
        let request_slice = postcard::to_slice(request, &mut serialized_request).map_err(|_| ())?;
        self.send_raw(request_slice)?;

        let data = self.recv_blocking_bytes()?;
        postcard::from_bytes(data).map_err(|_| ())
    }
}

impl IpcSend for VNodeChannel {
    fn send_raw(&mut self, bytes: &[u8]) -> core::result::Result<(), ()> {
        let res = syscall3(
            SYS_IPC_SEND,
            self.id as u64,
            bytes.as_ptr() as u64,
            bytes.len() as u64,
        );
        if res == SUCCESS { Ok(()) } else { Err(()) }
    }
}

impl IpcRecv for VNodeChannel {
    fn recv_raw(&mut self) -> Option<Vec<u8>> {
        match self.recv_non_blocking() {
            Ok(Some(data)) => Some(data),
            _ => None,
        }
    }
}
