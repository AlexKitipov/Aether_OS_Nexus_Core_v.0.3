extern crate alloc;

use alloc::collections::VecDeque;
use alloc::sync::Arc;
use alloc::vec;
use core::ptr::{read_volatile, write_volatile};
use spin::Mutex;
use x86_64::instructions::port::Port;

use crate::arch::x86_64::paging;
use crate::device::{
    Capability, CapabilitySet, Device, DeviceId, DeviceKind, InterruptHandler, IoDevice, IoResult,
    Rights, DEVICE_NET0,
};
use crate::kprintln;

pub const E1000_IRQ: u8 = 11;

const PCI_CONFIG_ADDRESS: u16 = 0xcf8;
const PCI_CONFIG_DATA: u16 = 0xcfc;
const PCI_VENDOR_INTEL: u16 = 0x8086;
const E1000_DEVICE_IDS: &[u16] = &[0x100e, 0x100f, 0x1015, 0x1016, 0x1017, 0x10d3];

const REG_CTRL: u32 = 0x0000;
const REG_EERD: u32 = 0x0014;
const REG_ICR: u32 = 0x00c0;
const REG_IMS: u32 = 0x00d0;
const REG_IMC: u32 = 0x00d8;
const REG_RCTL: u32 = 0x0100;
const REG_TCTL: u32 = 0x0400;
const REG_RDBAL: u32 = 0x2800;
const REG_RDBAH: u32 = 0x2804;
const REG_RDLEN: u32 = 0x2808;
const REG_RDH: u32 = 0x2810;
const REG_RDT: u32 = 0x2818;
const REG_TDBAL: u32 = 0x3800;
const REG_TDBAH: u32 = 0x3804;
const REG_TDLEN: u32 = 0x3808;
const REG_TDH: u32 = 0x3810;
const REG_TDT: u32 = 0x3818;
const REG_MTA: u32 = 0x5200;
const REG_RAL0: u32 = 0x5400;
const REG_RAH0: u32 = 0x5404;

const CTRL_SLU: u32 = 1 << 6;
const RCTL_EN: u32 = 1 << 1;
const RCTL_SBP: u32 = 1 << 2;
const RCTL_UPE: u32 = 1 << 3;
const RCTL_MPE: u32 = 1 << 4;
const RCTL_BAM: u32 = 1 << 15;
const RCTL_SECRC: u32 = 1 << 26;
const RCTL_BSIZE_2048: u32 = 0;
const TCTL_EN: u32 = 1 << 1;
const TCTL_PSP: u32 = 1 << 3;
const TCTL_CT_SHIFT: u32 = 4;
const TCTL_COLD_SHIFT: u32 = 12;
const INT_RXT0: u32 = 1 << 7;
const INT_RXDMT0: u32 = 1 << 4;
const INT_TXDW: u32 = 1 << 0;

const RX_DESC_COUNT: usize = 16;
const TX_DESC_COUNT: usize = 16;
const ETHERNET_MAX_FRAME: usize = 1518;
const DMA_BUF_SIZE: usize = 2048;

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

#[repr(C, packed)]
#[derive(Clone, Copy)]
struct RxDescriptor {
    addr: u64,
    length: u16,
    checksum: u16,
    status: u8,
    errors: u8,
    special: u16,
}

#[repr(C, packed)]
#[derive(Clone, Copy)]
struct TxDescriptor {
    addr: u64,
    length: u16,
    cso: u8,
    cmd: u8,
    status: u8,
    css: u8,
    special: u16,
}

#[derive(Debug, Clone, Copy)]
struct PciLocation {
    bus: u8,
    slot: u8,
    function: u8,
}

struct E1000State {
    mmio_base: u64,
    mmio_len: usize,
    mac: [u8; 6],
    rx_desc_virt: u64,
    tx_desc_virt: u64,
    rx_buffer_virt: [u64; RX_DESC_COUNT],
    tx_buffer_virt: [u64; TX_DESC_COUNT],
    rx_tail: usize,
    tx_tail: usize,
}

pub struct E1000NetDevice {
    state: Mutex<Option<E1000State>>,
}

impl E1000NetDevice {
    pub const fn new() -> Self {
        Self {
            state: Mutex::new(None),
        }
    }

    pub fn probe_and_init(&'static self) -> Option<()> {
        let mut state_guard = self.state.lock();
        if state_guard.is_some() {
            return Some(());
        }

        let probe = probe_e1000()?;
        let Some(rx_desc_region) = alloc_dma_region(RX_DESC_COUNT * core::mem::size_of::<RxDescriptor>(), 16) else {
            kprintln!("[kernel] e1000: failed to allocate RX descriptor ring");
            return None;
        };
        let Some(tx_desc_region) = alloc_dma_region(TX_DESC_COUNT * core::mem::size_of::<TxDescriptor>(), 16) else {
            kprintln!("[kernel] e1000: failed to allocate TX descriptor ring");
            return None;
        };

        let mut rx_buffer_virt = [0u64; RX_DESC_COUNT];
        let mut tx_buffer_virt = [0u64; TX_DESC_COUNT];
        let mut rx_buffer_phys = [0u64; RX_DESC_COUNT];
        let mut tx_buffer_phys = [0u64; TX_DESC_COUNT];
        for idx in 0..RX_DESC_COUNT {
            let Some(region) = alloc_dma_region(DMA_BUF_SIZE, 16) else {
                kprintln!("[kernel] e1000: failed to allocate RX DMA buffer");
                return None;
            };
            rx_buffer_virt[idx] = region.virt;
            rx_buffer_phys[idx] = region.phys;
        }
        for idx in 0..TX_DESC_COUNT {
            let Some(region) = alloc_dma_region(DMA_BUF_SIZE, 16) else {
                kprintln!("[kernel] e1000: failed to allocate TX DMA buffer");
                return None;
            };
            tx_buffer_virt[idx] = region.virt;
            tx_buffer_phys[idx] = region.phys;
        }

        let mmio_len = 128 * 1024;
        let mmio_base = map_mmio_window(probe.bar0, mmio_len);
        let mut state = E1000State {
            mmio_base,
            mmio_len,
            mac: [0; 6],
            rx_desc_virt: rx_desc_region.virt,
            tx_desc_virt: tx_desc_region.virt,
            rx_buffer_virt,
            tx_buffer_virt,
            rx_tail: RX_DESC_COUNT - 1,
            tx_tail: 0,
        };

        enable_pci_bus_mastering(probe.pci);
        state.mac = read_mac(&state);
        init_rx_ring(&state, rx_desc_region.phys, &rx_buffer_phys);
        init_tx_ring(&state, tx_desc_region.phys, &tx_buffer_phys);

        mmio_write(&state, REG_IMC, u32::MAX);
        let _ = mmio_read(&state, REG_ICR);
        mmio_write(&state, REG_CTRL, mmio_read(&state, REG_CTRL) | CTRL_SLU);
        for offset in (0..128).map(|idx| REG_MTA + idx * 4) {
            mmio_write(&state, offset, 0);
        }
        mmio_write(&state, REG_RAL0, u32::from_le_bytes([state.mac[0], state.mac[1], state.mac[2], state.mac[3]]));
        mmio_write(&state, REG_RAH0, u32::from_le_bytes([state.mac[4], state.mac[5], 0, 0]) | (1 << 31));
        mmio_write(&state, REG_RCTL, RCTL_EN | RCTL_SBP | RCTL_UPE | RCTL_MPE | RCTL_BAM | RCTL_SECRC | RCTL_BSIZE_2048);
        mmio_write(&state, REG_TCTL, TCTL_EN | TCTL_PSP | (0x10 << TCTL_CT_SHIFT) | (0x40 << TCTL_COLD_SHIFT));
        mmio_write(&state, REG_IMS, INT_RXT0 | INT_RXDMT0 | INT_TXDW);

        kprintln!(
            "[kernel] e1000: initialized pci={}.{}.{} bar0={:#x} mmio={:#x} mac={:02x}:{:02x}:{:02x}:{:02x}:{:02x}:{:02x}",
            probe.pci.bus,
            probe.pci.slot,
            probe.pci.function,
            probe.bar0,
            mmio_base,
            state.mac[0],
            state.mac[1],
            state.mac[2],
            state.mac[3],
            state.mac[4],
            state.mac[5]
        );

        *state_guard = Some(state);
        Some(())
    }

    pub fn acknowledge_interrupt(&self) {
        if let Some(state) = self.state.lock().as_ref() {
            let cause = mmio_read(state, REG_ICR);
            if cause != 0 {
                kprintln!("[kernel] e1000: irq ack cause={:#x}", cause);
            }
        }
    }

    pub fn is_initialized(&self) -> bool {
        self.state.lock().is_some()
    }
}

impl NetDevice for E1000NetDevice {
    fn send(&self, frame: &[u8]) {
        if frame.is_empty() || frame.len() > ETHERNET_MAX_FRAME {
            return;
        }

        let mut guard = self.state.lock();
        let Some(state) = guard.as_mut() else {
            return;
        };

        let tx_index = state.tx_tail;
        let buffer_ptr = state.tx_buffer_virt[tx_index] as *mut u8;
        unsafe {
            core::ptr::copy_nonoverlapping(frame.as_ptr(), buffer_ptr, frame.len());
        }

        let desc_ptr = state.tx_desc_virt as *mut TxDescriptor;
        let desc = unsafe { &mut *desc_ptr.add(tx_index) };
        desc.length = frame.len() as u16;
        desc.cso = 0;
        desc.cmd = 0x0b;
        desc.status = 0;
        desc.css = 0;
        desc.special = 0;

        state.tx_tail = (state.tx_tail + 1) % TX_DESC_COUNT;
        mmio_write(state, REG_TDT, state.tx_tail as u32);
    }

    fn receive(&self, buf: &mut [u8]) -> usize {
        let mut guard = self.state.lock();
        let Some(state) = guard.as_mut() else {
            return 0;
        };

        let next = (state.rx_tail + 1) % RX_DESC_COUNT;
        let desc_ptr = state.rx_desc_virt as *mut RxDescriptor;
        let desc = unsafe { &mut *desc_ptr.add(next) };
        if (desc.status & 0x01) == 0 {
            return 0;
        }

        let frame_len = desc.length as usize;
        let n = core::cmp::min(buf.len(), frame_len);
        let buffer_ptr = state.rx_buffer_virt[next] as *const u8;
        unsafe {
            core::ptr::copy_nonoverlapping(buffer_ptr, buf.as_mut_ptr(), n);
        }

        desc.status = 0;
        desc.length = 0;
        state.rx_tail = next;
        mmio_write(state, REG_RDT, state.rx_tail as u32);
        n
    }

    fn mac(&self) -> [u8; 6] {
        self.state
            .lock()
            .as_ref()
            .map(|state| state.mac)
            .unwrap_or([0x02, 0, 0, 0, 0, 1])
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
        DeviceKind::Network
    }

    fn capabilities(&self) -> CapabilitySet {
        vec![Capability {
            device: self.id,
            rights: Rights::READ.union(Rights::WRITE).union(Rights::INTERRUPT),
        }]
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

impl InterruptHandler for NetworkDeviceIo {
    fn handle_irq(&self) {
        E1000_NET0.acknowledge_interrupt();
        let _ = crate::network::with_stack(|stack| stack.poll_device());
    }
}

#[derive(Debug, Clone, Copy)]
struct E1000Probe {
    pci: PciLocation,
    bar0: u64,
}

fn probe_e1000() -> Option<E1000Probe> {
    for bus in 0..=255u8 {
        for slot in 0..32u8 {
            for function in 0..8u8 {
                let pci = PciLocation { bus, slot, function };
                let vendor = pci_read16(pci, 0x00);
                if vendor == 0xffff {
                    if function == 0 {
                        break;
                    }
                    continue;
                }
                let device = pci_read16(pci, 0x02);
                if vendor == PCI_VENDOR_INTEL && E1000_DEVICE_IDS.contains(&device) {
                    let bar0_raw = pci_read32(pci, 0x10);
                    if (bar0_raw & 0x1) != 0 {
                        kprintln!("[kernel] e1000: I/O BARs are not supported for device {:#x}", device);
                        return None;
                    }
                    return Some(E1000Probe {
                        pci,
                        bar0: (bar0_raw & 0xfffffff0) as u64,
                    });
                }
                if function == 0 && (pci_read8(pci, 0x0e) & 0x80) == 0 {
                    break;
                }
            }
        }
    }
    kprintln!("[kernel] e1000: no Intel e1000-compatible PCI NIC found");
    None
}

#[derive(Debug, Clone, Copy)]
struct DmaRegion {
    virt: u64,
    phys: u64,
}

fn alloc_dma_region(size: usize, align: u64) -> Option<DmaRegion> {
    let align = align.max(1);
    let raw_phys = paging::alloc_frame_range(size.saturating_add(align as usize));
    if raw_phys == 0 {
        return None;
    }
    let phys = (raw_phys + align - 1) & !(align - 1);
    let virt = paging::physical_memory_offset().map(|offset| offset + phys).unwrap_or(phys);
    paging::register_virt_mapping(virt, phys, size);
    unsafe { core::ptr::write_bytes(virt as *mut u8, 0, size) };
    Some(DmaRegion { virt, phys })
}

fn map_mmio_window(phys: u64, len: usize) -> u64 {
    if let Some(offset) = paging::physical_memory_offset() {
        let virt = offset + phys;
        paging::register_virt_mapping(virt, phys, len);
        virt
    } else {
        paging::register_virt_mapping(phys, phys, len);
        phys
    }
}

fn init_rx_ring(state: &E1000State, rx_desc_phys: u64, rx_buffer_phys: &[u64; RX_DESC_COUNT]) {
    let desc_ptr = state.rx_desc_virt as *mut RxDescriptor;
    for idx in 0..RX_DESC_COUNT {
        let buffer_phys = rx_buffer_phys[idx];
        let desc = unsafe { &mut *desc_ptr.add(idx) };
        *desc = RxDescriptor {
            addr: buffer_phys,
            length: 0,
            checksum: 0,
            status: 0,
            errors: 0,
            special: 0,
        };
    }

    mmio_write(state, REG_RDBAL, rx_desc_phys as u32);
    mmio_write(state, REG_RDBAH, (rx_desc_phys >> 32) as u32);
    mmio_write(state, REG_RDLEN, (RX_DESC_COUNT * core::mem::size_of::<RxDescriptor>()) as u32);
    mmio_write(state, REG_RDH, 0);
    mmio_write(state, REG_RDT, (RX_DESC_COUNT - 1) as u32);
}

fn init_tx_ring(state: &E1000State, tx_desc_phys: u64, tx_buffer_phys: &[u64; TX_DESC_COUNT]) {
    let desc_ptr = state.tx_desc_virt as *mut TxDescriptor;
    for idx in 0..TX_DESC_COUNT {
        let buffer_phys = tx_buffer_phys[idx];
        let desc = unsafe { &mut *desc_ptr.add(idx) };
        *desc = TxDescriptor {
            addr: buffer_phys,
            length: 0,
            cso: 0,
            cmd: 0,
            status: 1,
            css: 0,
            special: 0,
        };
    }

    mmio_write(state, REG_TDBAL, tx_desc_phys as u32);
    mmio_write(state, REG_TDBAH, (tx_desc_phys >> 32) as u32);
    mmio_write(state, REG_TDLEN, (TX_DESC_COUNT * core::mem::size_of::<TxDescriptor>()) as u32);
    mmio_write(state, REG_TDH, 0);
    mmio_write(state, REG_TDT, 0);
}

fn read_mac(state: &E1000State) -> [u8; 6] {
    let ral = mmio_read(state, REG_RAL0);
    let rah = mmio_read(state, REG_RAH0);
    if (rah & (1 << 31)) != 0 {
        return [
            (ral & 0xff) as u8,
            ((ral >> 8) & 0xff) as u8,
            ((ral >> 16) & 0xff) as u8,
            ((ral >> 24) & 0xff) as u8,
            (rah & 0xff) as u8,
            ((rah >> 8) & 0xff) as u8,
        ];
    }

    let mut mac = [0u8; 6];
    for word in 0..3u32 {
        mmio_write(state, REG_EERD, (word << 8) | 1);
        let mut value = 0;
        for _ in 0..10_000 {
            value = mmio_read(state, REG_EERD);
            if (value & (1 << 4)) != 0 {
                break;
            }
        }
        let data = ((value >> 16) & 0xffff) as u16;
        mac[(word as usize) * 2] = (data & 0xff) as u8;
        mac[(word as usize) * 2 + 1] = (data >> 8) as u8;
    }

    if mac == [0; 6] {
        [0x02, 0xae, 0x10, 0x00, 0x00, 0x01]
    } else {
        mac
    }
}

fn mmio_read(state: &E1000State, offset: u32) -> u32 {
    debug_assert!((offset as usize) + core::mem::size_of::<u32>() <= state.mmio_len);
    unsafe { read_volatile((state.mmio_base + offset as u64) as *const u32) }
}

fn mmio_write(state: &E1000State, offset: u32, value: u32) {
    debug_assert!((offset as usize) + core::mem::size_of::<u32>() <= state.mmio_len);
    unsafe { write_volatile((state.mmio_base + offset as u64) as *mut u32, value) }
}

fn pci_read32(pci: PciLocation, offset: u8) -> u32 {
    let address = 0x8000_0000u32
        | ((pci.bus as u32) << 16)
        | ((pci.slot as u32) << 11)
        | ((pci.function as u32) << 8)
        | ((offset as u32) & 0xfc);
    unsafe {
        let mut addr: Port<u32> = Port::new(PCI_CONFIG_ADDRESS);
        let mut data: Port<u32> = Port::new(PCI_CONFIG_DATA);
        addr.write(address);
        data.read()
    }
}

fn pci_write32(pci: PciLocation, offset: u8, value: u32) {
    let address = 0x8000_0000u32
        | ((pci.bus as u32) << 16)
        | ((pci.slot as u32) << 11)
        | ((pci.function as u32) << 8)
        | ((offset as u32) & 0xfc);
    unsafe {
        let mut addr: Port<u32> = Port::new(PCI_CONFIG_ADDRESS);
        let mut data: Port<u32> = Port::new(PCI_CONFIG_DATA);
        addr.write(address);
        data.write(value);
    }
}

fn pci_read16(pci: PciLocation, offset: u8) -> u16 {
    ((pci_read32(pci, offset) >> ((offset & 2) * 8)) & 0xffff) as u16
}

fn pci_read8(pci: PciLocation, offset: u8) -> u8 {
    ((pci_read32(pci, offset) >> ((offset & 3) * 8)) & 0xff) as u8
}

fn enable_pci_bus_mastering(pci: PciLocation) {
    let command = pci_read16(pci, 0x04) | 0x0002 | 0x0004;
    let mut reg = pci_read32(pci, 0x04);
    reg = (reg & 0xffff_0000) | command as u32;
    pci_write32(pci, 0x04, reg);
}

pub fn register_detected_device(manager: &mut crate::device::DeviceManager) -> Option<Arc<NetworkDeviceIo>> {
    E1000_NET0.probe_and_init()?;
    let device = Arc::new(NetworkDeviceIo::new(DEVICE_NET0, &E1000_NET0));
    manager.register_io(device.clone());
    manager.register_irq(E1000_IRQ, device.clone());
    let _ = crate::network::with_stack(|stack| stack.bind_device(&E1000_NET0));
    Some(device)
}

pub static VIRTIO_NET0: VirtIoNetDevice = VirtIoNetDevice::new([0x02, 0, 0, 0, 0, 1]);
pub static E1000_NET0: E1000NetDevice = E1000NetDevice::new();
