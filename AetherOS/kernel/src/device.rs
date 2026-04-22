#![allow(dead_code)]

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::sync::Arc;
use alloc::vec::Vec;
use adi::interface::{ADIInterface, DeviceInfo as ADIDeviceInfo};
use core::cmp::Ordering;
use spin::Mutex;

use crate::kprintln;

pub type DeviceId = u64;
pub type IoResult<T> = core::result::Result<T, IoError>;

pub const DEVICE_TIMER: DeviceId = 1;
pub const DEVICE_SERIAL: DeviceId = 2;
pub const DEVICE_FRAMEBUFFER: DeviceId = 3;
pub const DEVICE_NET0: DeviceId = 4;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum DeviceKind {
    Timer,
    Serial,
    Framebuffer,
    Network,
    Unknown,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IoError {
    PermissionDenied,
    DeviceNotFound,
    Unsupported,
    Fault,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Right {
    Read,
    Write,
    Manage,
    Interrupt,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Rights(u8);

impl Rights {
    pub const NONE: Self = Self(0);
    pub const READ: Self = Self(1 << 0);
    pub const WRITE: Self = Self(1 << 1);
    pub const MANAGE: Self = Self(1 << 2);
    pub const INTERRUPT: Self = Self(1 << 3);

    #[inline]
    pub const fn union(self, other: Self) -> Self {
        Self(self.0 | other.0)
    }

    #[inline]
    pub const fn allows(self, right: Right) -> bool {
        let bit = match right {
            Right::Read => Self::READ.0,
            Right::Write => Self::WRITE.0,
            Right::Manage => Self::MANAGE.0,
            Right::Interrupt => Self::INTERRUPT.0,
        };
        (self.0 & bit) != 0
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Capability {
    pub device: DeviceId,
    pub rights: Rights,
}

pub type CapabilitySet = Vec<Capability>;

#[derive(Debug, Clone, Default)]
pub struct VNode {
    pub caps: CapabilitySet,
}

impl VNode {
    pub fn has_cap(&self, dev: DeviceId, right: Right) -> bool {
        check_cap(self, dev, right)
    }
}

pub fn check_cap(vnode: &VNode, dev: DeviceId, right: Right) -> bool {
    vnode
        .caps
        .iter()
        .any(|cap| cap.device == dev && cap.rights.allows(right))
}

pub trait Device: Send + Sync {
    fn id(&self) -> DeviceId;
    fn kind(&self) -> DeviceKind;
    fn capabilities(&self) -> CapabilitySet;
}

pub trait IoDevice: Device {
    fn read(&self, buf: &mut [u8]) -> IoResult<usize>;
    fn write(&self, buf: &[u8]) -> IoResult<usize>;
}

pub trait InterruptHandler: Send + Sync {
    fn handle_irq(&self);
}

pub struct DeviceManager {
    devices: BTreeMap<DeviceId, Arc<dyn Device>>,
    io_devices: BTreeMap<DeviceId, Arc<dyn IoDevice>>,
    irq_handlers: BTreeMap<u8, Arc<dyn InterruptHandler>>,
}

impl DeviceManager {
    pub fn new() -> Self {
        Self {
            devices: BTreeMap::new(),
            io_devices: BTreeMap::new(),
            irq_handlers: BTreeMap::new(),
        }
    }

    pub fn register(&mut self, dev: Arc<dyn Device>) {
        self.devices.insert(dev.id(), dev);
    }

    pub fn register_io(&mut self, dev: Arc<dyn IoDevice>) {
        let metadata: Arc<dyn Device> = Arc::new(DeviceMetadata::from_io(&*dev));
        self.devices.insert(metadata.id(), metadata);
        self.io_devices.insert(dev.id(), dev);
    }

    pub fn get(&self, id: DeviceId) -> Option<Arc<dyn Device>> {
        self.devices.get(&id).cloned()
    }

    pub fn get_io(&self, id: DeviceId) -> Option<Arc<dyn IoDevice>> {
        self.io_devices.get(&id).cloned()
    }

    pub fn register_irq(&mut self, irq: u8, handler: Arc<dyn InterruptHandler>) {
        self.irq_handlers.insert(irq, handler);
    }

    pub fn irq_handler(&self, irq: u8) -> Option<Arc<dyn InterruptHandler>> {
        self.irq_handlers.get(&irq).cloned()
    }

    pub fn discovered_devices(&self) -> alloc::vec::Vec<DeviceId> {
        self.devices.keys().copied().collect()
    }
}

#[derive(Debug, Clone)]
struct DeviceMetadata {
    id: DeviceId,
    kind: DeviceKind,
    caps: CapabilitySet,
}

impl DeviceMetadata {
    fn from_io(dev: &dyn IoDevice) -> Self {
        Self {
            id: dev.id(),
            kind: dev.kind(),
            caps: dev.capabilities(),
        }
    }
}

impl Device for DeviceMetadata {
    fn id(&self) -> DeviceId {
        self.id
    }

    fn kind(&self) -> DeviceKind {
        self.kind
    }

    fn capabilities(&self) -> CapabilitySet {
        self.caps.clone()
    }
}

static DEVICE_MANAGER: Mutex<Option<DeviceManager>> = Mutex::new(None);

pub fn init() {
    *DEVICE_MANAGER.lock() = Some(DeviceManager::new());
    kprintln!("[kernel] device-manager: initialized.");
}

pub fn with_manager<R>(f: impl FnOnce(&mut DeviceManager) -> R) -> Option<R> {
    let mut guard = DEVICE_MANAGER.lock();
    guard.as_mut().map(f)
}

pub fn boot_discover_devices() {
    kprintln!("[kernel] device-discovery: begin.");
    kprintln!("[kernel] device-discovery: pci probe scheduled.");

    let _ = with_manager(|manager| {
        manager.register_io(Arc::new(crate::drivers::timer::TimerDriver::new()));
        let _ = probe_device(ADIDeviceInfo::new("timer-driver"));
        manager.register_irq(0, Arc::new(crate::drivers::timer::TimerDriver::new()));
        manager.register_io(Arc::new(crate::drivers::serial::SerialDevice::new()));
        let _ = probe_device(ADIDeviceInfo::new("serial-driver"));
        manager.register_io(Arc::new(crate::drivers::framebuffer::FramebufferDevice::new()));
        let _ = probe_device(ADIDeviceInfo::new("framebuffer-driver"));
        manager.register_io(Arc::new(crate::drivers::net::NetworkDeviceIo::new(
            DEVICE_NET0,
            &crate::drivers::net::VIRTIO_NET0,
        )));
        let _ = probe_device(ADIDeviceInfo::new("network-driver"));

        let _ = crate::network::with_stack(|stack| {
            stack.bind_device(&crate::drivers::net::VIRTIO_NET0);
        });

        let list = manager.discovered_devices();
        for id in list {
            kprintln!("[kernel] device-discovery: registered device id={}", id);
        }
    });

    kprintln!("[kernel] device-discovery: done.");
}

fn probe_device(device: ADIDeviceInfo) -> bool {
    ADIInterface::load_driver(device).is_ok()
}

pub fn vnode_caps_from_task(task_id: u64) -> VNode {
    let mut vnode = VNode::default();

    let _ = with_manager(|manager| {
        for id in manager.discovered_devices() {
            let mut rights = Rights::NONE;

            if crate::caps::Capability::TimeRead.check(task_id) && id == DEVICE_TIMER {
                rights = rights.union(Rights::READ);
            }

            if crate::caps::Capability::LogWrite.check(task_id) && id == DEVICE_SERIAL {
                rights = rights.union(Rights::WRITE).union(Rights::READ);
            }

            if crate::caps::Capability::StorageAccess.check(task_id) && id == DEVICE_FRAMEBUFFER {
                rights = rights.union(Rights::WRITE).union(Rights::READ);
            }

            if crate::caps::Capability::NetworkAccess.check(task_id) && id == DEVICE_NET0 {
                rights = rights.union(Rights::WRITE).union(Rights::READ);
            }

            if rights != Rights::NONE {
                vnode.caps.push(Capability { device: id, rights });
            }
        }
    });

    vnode
}

impl Ord for Rights {
    fn cmp(&self, other: &Self) -> Ordering {
        self.0.cmp(&other.0)
    }
}

impl PartialOrd for Rights {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}
