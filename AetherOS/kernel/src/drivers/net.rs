extern crate alloc;

use alloc::collections::VecDeque;
use spin::Mutex;

use crate::device::{CapabilitySet, Device, DeviceId, DeviceKind, IoDevice, IoError, IoResult};

pub trait NetDevice: Send + Sync {
    fn send(&self, frame: &[u8]);
    fn receive(&self, buf: &mut [u8]) -> usize;
    fn mac(&self) -> [u8; 6];
}

#[derive(Debug)]
pub struct VirtIoNetDevice {
    mac: [u8; 6],
    rx_queue: Mutex<VecDeque<alloc::vec::Vec<u8>>>,
    tx_queue: Mutex<VecDeque<alloc::vec::Vec<u8>>>,
}

impl VirtIoNetDevice {
    pub const fn new(mac: [u8; 6]) -> Self {
        Self {
            mac,
            rx_queue: Mutex::new(VecDeque::new()),
            tx_queue: Mutex::new(VecDeque::new()),
        }
    }

    pub fn inject_rx_frame(&self, frame: &[u8]) {
        self.rx_queue.lock().push_back(frame.to_vec());
    }

    pub fn take_tx_frame(&self) -> Option<alloc::vec::Vec<u8>> {
        self.tx_queue.lock().pop_front()
    }
}

impl NetDevice for VirtIoNetDevice {
    fn send(&self, frame: &[u8]) {
        self.tx_queue.lock().push_back(frame.to_vec());
    }

    fn receive(&self, buf: &mut [u8]) -> usize {
        let Some(frame) = self.rx_queue.lock().pop_front() else {
            return 0;
        };
        let n = core::cmp::min(buf.len(), frame.len());
        buf[..n].copy_from_slice(&frame[..n]);
        n
    }

    fn mac(&self) -> [u8; 6] {
        self.mac
    }
}

#[derive(Debug)]
pub struct E1000NetDevice {
    mac: [u8; 6],
}

impl E1000NetDevice {
    pub const fn new(mac: [u8; 6]) -> Self {
        Self { mac }
    }
}

impl NetDevice for E1000NetDevice {
    fn send(&self, _frame: &[u8]) {}

    fn receive(&self, _buf: &mut [u8]) -> usize {
        0
    }

    fn mac(&self) -> [u8; 6] {
        self.mac
    }
}

pub struct NetworkDeviceIo {
    id: DeviceId,
    dev: &'static dyn NetDevice,
}

impl NetworkDeviceIo {
    pub const fn new(id: DeviceId, dev: &'static dyn NetDevice) -> Self {
        Self { id, dev }
    }
}

impl Device for NetworkDeviceIo {
    fn id(&self) -> DeviceId {
        self.id
    }

    fn kind(&self) -> DeviceKind {
        DeviceKind::Unknown
    }

    fn capabilities(&self) -> CapabilitySet {
        alloc::vec![]
    }
}

impl IoDevice for NetworkDeviceIo {
    fn read(&self, buf: &mut [u8]) -> IoResult<usize> {
        Ok(self.dev.receive(buf))
    }

    fn write(&self, buf: &[u8]) -> IoResult<usize> {
        self.dev.send(buf);
        Ok(buf.len())
    }
}

pub static VIRTIO_NET0: VirtIoNetDevice = VirtIoNetDevice::new([0x02, 0, 0, 0, 0, 1]);
