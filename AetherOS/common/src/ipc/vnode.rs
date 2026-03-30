// common/src/ipc/vnode.rs


extern crate alloc;

use alloc::vec::Vec;
use crate::ipc::{IpcSend, IpcRecv};
use crate::syscall::{
    syscall3, E_ACC_DENIED, E_ERROR, SUCCESS, SYS_IPC_RECV, SYS_IPC_RECV_NONBLOCKING, SYS_IPC_SEND,
};

pub struct VNodeChannel {
    pub id: u32,
    buffer: [u8; 4096],
}

impl VNodeChannel {
    pub fn new(id: u32) -> Self {
        Self { id, buffer: [0; 4096] }
    }

    pub fn recv_blocking(&mut self) -> core::result::Result<Vec<u8>, ()> {
        loop {
            let len = syscall3(
                SYS_IPC_RECV,
                self.id as u64,
                self.buffer.as_mut_ptr() as u64,
                self.buffer.len() as u64 // Pass max capacity
            );
            match len {
                SUCCESS => {
                    // Blocking receive may transiently return 0 around reschedule boundaries; retry.
                }
                E_ERROR | E_ACC_DENIED => return Err(()),
                l => {
                    let msg_len = l as usize;
                    if msg_len > self.buffer.len() {
                        return Err(());
                    }
                    return Ok(self.buffer[..msg_len].to_vec());
                }
            }
        }
    }

    pub fn recv_non_blocking(&mut self) -> core::result::Result<Option<Vec<u8>>, ()> {
        let len = syscall3(
            SYS_IPC_RECV_NONBLOCKING,
            self.id as u64,
            self.buffer.as_mut_ptr() as u64,
            self.buffer.len() as u64 // Pass max capacity
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

    pub fn send_and_recv<Req: serde::Serialize, Resp: serde::de::DeserializeOwned>(
        &mut self, request: &Req
    ) -> core::result::Result<Resp, ()> {
        let serialized_request = postcard::to_allocvec(request).map_err(|_| ())?;
        self.send_raw(&serialized_request)?;
        
        // After sending, immediately try to receive the response.
        // This assumes a synchronous request-response pattern.
        match self.recv_blocking() {
            Ok(data) => postcard::from_bytes(&data).map_err(|_| ()),
            Err(_) => Err(()),
        }
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
