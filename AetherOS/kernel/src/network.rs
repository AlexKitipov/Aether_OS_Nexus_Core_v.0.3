#![allow(dead_code)]

extern crate alloc;

use alloc::collections::{BTreeMap, VecDeque};
use alloc::vec::Vec;
use core::cmp::min;
use core::net::Ipv4Addr;
use spin::Mutex;

use crate::drivers::net::NetDevice;

pub const ETHERTYPE_ARP: u16 = 0x0806;
pub const ETHERTYPE_IPV4: u16 = 0x0800;
pub const IPV4_PROTO_ICMP: u8 = 0x01;
pub const IPV4_PROTO_UDP: u8 = 0x11;

#[derive(Debug, Clone, Copy)]
pub struct EthernetFrame<'a> {
    pub dst: [u8; 6],
    pub src: [u8; 6],
    pub ethertype: u16,
    pub payload: &'a [u8],
}

impl<'a> EthernetFrame<'a> {
    pub fn parse(buf: &'a [u8]) -> Option<Self> {
        if buf.len() < 14 {
            return None;
        }

        let mut dst = [0u8; 6];
        dst.copy_from_slice(&buf[0..6]);
        let mut src = [0u8; 6];
        src.copy_from_slice(&buf[6..12]);
        let ethertype = u16::from_be_bytes([buf[12], buf[13]]);

        Some(Self {
            dst,
            src,
            ethertype,
            payload: &buf[14..],
        })
    }

    pub fn encode(&self, out: &mut [u8]) -> Option<usize> {
        let needed = 14 + self.payload.len();
        if out.len() < needed {
            return None;
        }

        out[0..6].copy_from_slice(&self.dst);
        out[6..12].copy_from_slice(&self.src);
        out[12..14].copy_from_slice(&self.ethertype.to_be_bytes());
        out[14..needed].copy_from_slice(self.payload);
        Some(needed)
    }
}

#[derive(Debug, Clone, Copy)]
pub struct ArpCacheEntry {
    pub mac: [u8; 6],
    pub updated_at_ticks: u64,
}

#[derive(Debug, Default)]
pub struct ArpCache {
    entries: BTreeMap<Ipv4Addr, ArpCacheEntry>,
}

impl ArpCache {
    pub fn lookup(&self, ip: Ipv4Addr) -> Option<[u8; 6]> {
        self.entries.get(&ip).map(|entry| entry.mac)
    }

    pub fn upsert(&mut self, ip: Ipv4Addr, mac: [u8; 6], now_ticks: u64) {
        self.entries.insert(
            ip,
            ArpCacheEntry {
                mac,
                updated_at_ticks: now_ticks,
            },
        );
    }
}

#[derive(Debug, Clone, Copy)]
pub struct Ipv4Packet<'a> {
    pub src: Ipv4Addr,
    pub dst: Ipv4Addr,
    pub protocol: u8,
    pub payload: &'a [u8],
}

impl<'a> Ipv4Packet<'a> {
    pub fn parse(buf: &'a [u8]) -> Option<Self> {
        if buf.len() < 20 {
            return None;
        }
        let version = buf[0] >> 4;
        let ihl_words = (buf[0] & 0x0f) as usize;
        let ihl = ihl_words * 4;
        if version != 4 || ihl < 20 || buf.len() < ihl {
            return None;
        }

        let total_len = u16::from_be_bytes([buf[2], buf[3]]) as usize;
        if total_len < ihl || total_len > buf.len() {
            return None;
        }

        if ipv4_checksum(&buf[..ihl]) != 0 {
            return None;
        }

        let src = Ipv4Addr::new(buf[12], buf[13], buf[14], buf[15]);
        let dst = Ipv4Addr::new(buf[16], buf[17], buf[18], buf[19]);

        Some(Self {
            src,
            dst,
            protocol: buf[9],
            payload: &buf[ihl..total_len],
        })
    }
}

#[derive(Debug, Clone, Copy)]
pub struct UdpPacket<'a> {
    pub src_port: u16,
    pub dst_port: u16,
    pub payload: &'a [u8],
}

impl<'a> UdpPacket<'a> {
    pub fn parse(buf: &'a [u8]) -> Option<Self> {
        if buf.len() < 8 {
            return None;
        }
        let src_port = u16::from_be_bytes([buf[0], buf[1]]);
        let dst_port = u16::from_be_bytes([buf[2], buf[3]]);
        let len = u16::from_be_bytes([buf[4], buf[5]]) as usize;
        if len < 8 || len > buf.len() {
            return None;
        }

        Some(Self {
            src_port,
            dst_port,
            payload: &buf[8..len],
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NetRights {
    Send,
    Receive,
    SendReceive,
}

impl NetRights {
    fn can_send(self) -> bool {
        matches!(self, Self::Send | Self::SendReceive)
    }

    fn can_recv(self) -> bool {
        matches!(self, Self::Receive | Self::SendReceive)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NetCapability {
    pub local_port: u16,
    pub remote_addr: Option<Ipv4Addr>,
    pub remote_port: Option<u16>,
    pub rights: NetRights,
}

impl NetCapability {
    pub fn allows_send(&self, local_port: u16, remote_addr: Ipv4Addr, remote_port: u16) -> bool {
        if !self.rights.can_send() || (self.local_port != 0 && self.local_port != local_port) {
            return false;
        }

        if let Some(allowed_addr) = self.remote_addr {
            if allowed_addr != remote_addr {
                return false;
            }
        }

        if let Some(allowed_port) = self.remote_port {
            if allowed_port != remote_port {
                return false;
            }
        }

        true
    }

    pub fn allows_recv(&self, local_port: u16) -> bool {
        self.rights.can_recv() && (self.local_port == 0 || self.local_port == local_port)
    }
}

#[derive(Debug, Clone)]
pub struct UdpDatagram {
    pub src: Ipv4Addr,
    pub dst: Ipv4Addr,
    pub src_port: u16,
    pub dst_port: u16,
    pub payload: Vec<u8>,
}

#[derive(Debug, Default)]
struct UdpSocketTable {
    rx: BTreeMap<u16, VecDeque<UdpDatagram>>,
}

impl UdpSocketTable {
    fn enqueue(&mut self, dgram: UdpDatagram) {
        self.rx.entry(dgram.dst_port).or_default().push_back(dgram);
    }

    fn recv(&mut self, local_port: u16, out: &mut [u8]) -> Option<usize> {
        let queue = self.rx.get_mut(&local_port)?;
        let pkt = queue.pop_front()?;
        let n = min(out.len(), pkt.payload.len());
        out[..n].copy_from_slice(&pkt.payload[..n]);
        Some(n)
    }
}

#[derive(Debug, Clone)]
pub struct SecureChannel {
    pub key: [u8; 32],
    pub nonce: u64,
}

impl SecureChannel {
    pub fn seal(&mut self, plaintext: &[u8]) -> Vec<u8> {
        // Minimal placeholder envelope for Step 12: nonce + payload.
        // TODO: replace with ChaCha20-Poly1305 once crypto crate is wired in no_std profile.
        let mut out = Vec::with_capacity(8 + plaintext.len());
        out.extend_from_slice(&self.nonce.to_be_bytes());
        out.extend_from_slice(plaintext);
        self.nonce = self.nonce.wrapping_add(1);
        out
    }

    pub fn open(&self, ciphertext: &[u8]) -> Option<Vec<u8>> {
        if ciphertext.len() < 8 {
            return None;
        }
        Some(ciphertext[8..].to_vec())
    }
}

pub struct NetworkStack {
    device: Option<&'static dyn NetDevice>,
    pub local_mac: [u8; 6],
    pub local_ip: Ipv4Addr,
    arp: ArpCache,
    udp: UdpSocketTable,
    caps: BTreeMap<u64, Vec<NetCapability>>,
}

impl Default for NetworkStack {
    fn default() -> Self {
        Self {
            device: None,
            local_mac: [0x02, 0, 0, 0, 0, 1],
            local_ip: Ipv4Addr::new(10, 0, 2, 15),
            arp: ArpCache::default(),
            udp: UdpSocketTable::default(),
            caps: BTreeMap::new(),
        }
    }
}

impl NetworkStack {
    pub fn bind_device(&mut self, dev: &'static dyn NetDevice) {
        self.local_mac = dev.mac();
        self.device = Some(dev);
    }

    pub fn grant_capability(&mut self, task_id: u64, cap: NetCapability) {
        self.caps.entry(task_id).or_default().push(cap);
    }

    pub fn poll_device(&mut self) {
        let Some(device) = self.device else {
            return;
        };

        let mut frame_buf = [0u8; 1518];
        loop {
            let len = device.receive(&mut frame_buf);
            if len == 0 {
                break;
            }
            self.ingest_ethernet_frame(&frame_buf[..len]);
        }
    }

    pub fn transmit_frame(&mut self, dst: [u8; 6], ethertype: u16, payload: &[u8]) -> Result<usize, ()> {
        let Some(device) = self.device else {
            return Err(());
        };
        let frame = EthernetFrame {
            dst,
            src: self.local_mac,
            ethertype,
            payload,
        };
        let mut out = [0u8; 1518];
        let len = frame.encode(&mut out).ok_or(())?;
        device.send(&out[..len]);
        Ok(len)
    }

    fn ingest_ethernet_frame(&mut self, buf: &[u8]) {
        let Some(frame) = EthernetFrame::parse(buf) else {
            return;
        };
        if frame.dst != self.local_mac && frame.dst != [0xff; 6] {
            return;
        }

        match frame.ethertype {
            ETHERTYPE_ARP => self.ingest_arp(frame.payload, frame.src),
            ETHERTYPE_IPV4 => self.ingest_ipv4(frame.payload, frame.src),
            _ => {}
        }
    }

    fn ingest_arp(&mut self, payload: &[u8], ethernet_src: [u8; 6]) {
        if payload.len() < 28 {
            return;
        }
        let hardware_type = u16::from_be_bytes([payload[0], payload[1]]);
        let protocol_type = u16::from_be_bytes([payload[2], payload[3]]);
        let hardware_len = payload[4];
        let protocol_len = payload[5];
        if hardware_type != 1 || protocol_type != ETHERTYPE_IPV4 || hardware_len != 6 || protocol_len != 4 {
            return;
        }

        let op = u16::from_be_bytes([payload[6], payload[7]]);
        let sender_mac = [payload[8], payload[9], payload[10], payload[11], payload[12], payload[13]];
        let sender_ip = Ipv4Addr::new(payload[14], payload[15], payload[16], payload[17]);
        let target_ip = Ipv4Addr::new(payload[24], payload[25], payload[26], payload[27]);
        self.arp.upsert(sender_ip, sender_mac, 0);

        if op == 1 && target_ip == self.local_ip {
            let mut reply = [0u8; 28];
            reply[0..2].copy_from_slice(&1u16.to_be_bytes());
            reply[2..4].copy_from_slice(&ETHERTYPE_IPV4.to_be_bytes());
            reply[4] = 6;
            reply[5] = 4;
            reply[6..8].copy_from_slice(&2u16.to_be_bytes());
            reply[8..14].copy_from_slice(&self.local_mac);
            reply[14..18].copy_from_slice(&self.local_ip.octets());
            reply[18..24].copy_from_slice(&sender_mac);
            reply[24..28].copy_from_slice(&sender_ip.octets());
            let _ = self.transmit_frame(ethernet_src, ETHERTYPE_ARP, &reply);
        }
    }

    fn ingest_ipv4(&mut self, payload: &[u8], ethernet_src: [u8; 6]) {
        let Some(packet) = Ipv4Packet::parse(payload) else {
            return;
        };
        self.arp.upsert(packet.src, ethernet_src, 0);
        if packet.dst != self.local_ip {
            return;
        }
        if packet.protocol == IPV4_PROTO_UDP {
            if let Some(udp) = UdpPacket::parse(packet.payload) {
                self.udp.enqueue(UdpDatagram {
                    src: packet.src,
                    dst: packet.dst,
                    src_port: udp.src_port,
                    dst_port: udp.dst_port,
                    payload: udp.payload.to_vec(),
                });
            }
        }
    }

    fn send_arp_request(&mut self, target_ip: Ipv4Addr) {
        let mut arp = [0u8; 28];
        arp[0..2].copy_from_slice(&1u16.to_be_bytes());
        arp[2..4].copy_from_slice(&ETHERTYPE_IPV4.to_be_bytes());
        arp[4] = 6;
        arp[5] = 4;
        arp[6..8].copy_from_slice(&1u16.to_be_bytes());
        arp[8..14].copy_from_slice(&self.local_mac);
        arp[14..18].copy_from_slice(&self.local_ip.octets());
        arp[18..24].copy_from_slice(&[0; 6]);
        arp[24..28].copy_from_slice(&target_ip.octets());
        let _ = self.transmit_frame([0xff; 6], ETHERTYPE_ARP, &arp);
    }

    fn send_udp_frame(&mut self, src_port: u16, dst_ip: Ipv4Addr, dst_port: u16, payload: &[u8]) -> Result<usize, ()> {
        let dst_mac = match self.arp.lookup(dst_ip) {
            Some(mac) => mac,
            None => {
                self.send_arp_request(dst_ip);
                return Err(());
            }
        };

        let udp_len = 8 + payload.len();
        let ipv4_len = 20 + udp_len;
        let mut packet = Vec::with_capacity(ipv4_len);
        packet.resize(ipv4_len, 0);
        packet[0] = 0x45;
        packet[2..4].copy_from_slice(&(ipv4_len as u16).to_be_bytes());
        packet[8] = 64;
        packet[9] = IPV4_PROTO_UDP;
        packet[12..16].copy_from_slice(&self.local_ip.octets());
        packet[16..20].copy_from_slice(&dst_ip.octets());
        let checksum = ipv4_checksum(&packet[..20]);
        packet[10..12].copy_from_slice(&checksum.to_be_bytes());
        packet[20..22].copy_from_slice(&src_port.to_be_bytes());
        packet[22..24].copy_from_slice(&dst_port.to_be_bytes());
        packet[24..26].copy_from_slice(&(udp_len as u16).to_be_bytes());
        packet[28..].copy_from_slice(payload);
        self.transmit_frame(dst_mac, ETHERTYPE_IPV4, &packet)?;
        Ok(payload.len())
    }

    pub fn udp_send(
        &mut self,
        task_id: u64,
        src_port: u16,
        dst_ip: Ipv4Addr,
        dst_port: u16,
        payload: &[u8],
    ) -> Result<usize, ()> {
        if !self.task_can_send(task_id, src_port, dst_ip, dst_port) {
            return Err(());
        }

        if self.device.is_some() && dst_ip != self.local_ip {
            return self.send_udp_frame(src_port, dst_ip, dst_port, payload);
        }

        self.udp.enqueue(UdpDatagram {
            src: self.local_ip,
            dst: dst_ip,
            src_port,
            dst_port,
            payload: payload.to_vec(),
        });
        Ok(payload.len())
    }

    pub fn udp_recv(&mut self, task_id: u64, local_port: u16, out: &mut [u8]) -> Result<usize, ()> {
        if !self.task_can_recv(task_id, local_port) {
            return Err(());
        }

        self.udp.recv(local_port, out).ok_or(())
    }

    fn task_can_send(&self, task_id: u64, src_port: u16, dst_ip: Ipv4Addr, dst_port: u16) -> bool {
        self.caps
            .get(&task_id)
            .map(|caps| {
                caps.iter()
                    .any(|cap| cap.allows_send(src_port, dst_ip, dst_port))
            })
            .unwrap_or(false)
    }

    fn task_can_recv(&self, task_id: u64, local_port: u16) -> bool {
        self.caps
            .get(&task_id)
            .map(|caps| caps.iter().any(|cap| cap.allows_recv(local_port)))
            .unwrap_or(false)
    }
}

static NETWORK_STACK: Mutex<Option<NetworkStack>> = Mutex::new(None);

pub fn init() {
    let mut stack = NetworkStack::default();
    stack.grant_capability(0, NetCapability {
        local_port: 0,
        remote_addr: None,
        remote_port: None,
        rights: NetRights::SendReceive,
    });
    *NETWORK_STACK.lock() = Some(stack);
}

pub fn with_stack<R>(f: impl FnOnce(&mut NetworkStack) -> R) -> Option<R> {
    let mut guard = NETWORK_STACK.lock();
    guard.as_mut().map(f)
}

pub fn ipv4_checksum(header: &[u8]) -> u16 {
    let mut sum: u32 = 0;

    let mut idx = 0;
    while idx + 1 < header.len() {
        if idx == 10 {
            idx += 2;
            continue;
        }
        let w = u16::from_be_bytes([header[idx], header[idx + 1]]) as u32;
        sum = sum.wrapping_add(w);
        idx += 2;
    }

    if idx < header.len() {
        sum = sum.wrapping_add((header[idx] as u32) << 8);
    }

    while (sum >> 16) != 0 {
        sum = (sum & 0xffff) + (sum >> 16);
    }

    !(sum as u16)
}
