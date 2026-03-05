#![no_std]

extern crate alloc;
use alloc::vec::Vec;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NetError {
    Unsupported,
}

#[derive(Debug, Default)]
pub struct NetClient;

impl NetClient {
    pub fn new() -> Self {
        Self
    }

    pub fn open_udp_socket(&mut self, _port: u16) -> Result<u32, NetError> {
        Ok(0)
    }

    pub fn send_to(
        &self,
        _socket_handle: u32,
        _ip_address: [u8; 4],
        _port: u16,
        _payload: Vec<u8>,
    ) -> Result<(), NetError> {
        Ok(())
    }

    pub fn recv(&self, _socket_handle: u32) -> Result<Vec<u8>, NetError> {
        Err(NetError::Unsupported)
    }
}
