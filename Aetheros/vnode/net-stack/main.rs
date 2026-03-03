#![no_std]
#![no_main]

extern crate alloc;

use core::panic::PanicInfo;
use alloc::vec::Vec;
use alloc::collections::BTreeMap;
use alloc::format;

use smoltcp::iface::{Config, Interface, SocketSet, QueryInterface};
use smoltcp::phy::Checksum;
use smoltcp::socket::{TcpSocket, UdpSocket, Socket};
use smoltcp::wire::{EthernetAddress, HardwareAddress, IpAddress, IpCidr, Ipv4Address, ETHERNET_MTU};
use smoltcp::time::Instant;

use crate::ipc::vnode::VNodeChannel;
use crate::syscall::{syscall2, syscall3, SYS_LOG, SUCCESS, E_ERROR, SYS_TIME};
use crate::ipc::net_ipc::{NetPacketMsg, NetStackRequest, NetStackResponse};

mod aethernet_device;
use aethernet_device::AetherNetDevice;

// Temporary log function for V-Nodes
fn log(msg: &str) {
    unsafe {
        let res = syscall3(
            SYS_LOG,
            msg.as_ptr() as u64,
            msg.len() as u64,
            0 // arg3 is unused for SYS_LOG
        );
        if res != SUCCESS { /* Handle log error, maybe panic or fall back */ }
    }
}

// Get current time from kernel
fn get_current_time_ms() -> u64 {
    // Assuming SYS_TIME returns ticks, convert to ms for smoltcp Instant
    unsafe { syscall2(SYS_TIME, 0, 0) * 10 } // Assuming 1 tick = 10 ms for demo
}

#[no_mangle]
pub extern "C" fn _start() -> ! {
    // Channel for requests from other V-Nodes (Socket API)
    let mut own_chan = VNodeChannel::new(3);
    // Channel for data plane communication with net-bridge (RxPackets, TxPacketAcks)
    let mut bridge_data_chan = VNodeChannel::new(2);

    log("AetherNet Service V-Node starting up...");

    // 1. Initialize AetherNetDevice to interact with the net-bridge driver
    // Pass the channel ID for net-bridge communication
    let mut device = AetherNetDevice::new(0, bridge_data_chan.id);

    // 2. Configure smoltcp interface
    let ethernet_addr = EthernetAddress([0x02, 0x00, 0x00, 0x00, 0x00, 0x01]);
    let config = Config::new(HardwareAddress::Ethernet(ethernet_addr));
    let mut iface = Interface::new(config, &mut device, Instant::from_millis(get_current_time_ms()));

    // Assign a static IP address
    iface.update_ip_addrs(|addrs| {
        addrs.push(IpCidr::new(IpAddress::v4(10, 0, 2, 15), 24)).unwrap();
    });
    log(&alloc::format!("AetherNet: IP Address set to {}", IpAddress::v4(10,0,2,15)));

    // 3. Initialize smoltcp SocketSet
    let mut sockets_storage = Vec::new();
    let mut sockets = SocketSet::new(sockets_storage);

    // 4. Socket Management
    let mut next_socket_handle: u32 = 1;
    let mut smoltcp_sockets_map: BTreeMap<u32, smoltcp::socket::SocketHandle> = BTreeMap::new(); // Maps our handle to smoltcp's

    // Main event loop for the network stack
    loop {
        let timestamp = Instant::from_millis(get_current_time_ms());

        // --- Handle Incoming Packets from net-bridge V-Node via IPC --- (from net-bridge to aethernet_device)
        if let Ok(Some(net_msg_data)) = bridge_data_chan.recv_non_blocking() { // Check for messages from net-bridge
            if let Ok(net_packet_msg) = postcard::from_bytes::<NetPacketMsg>(&net_msg_data) {
                match net_packet_msg {
                    NetPacketMsg::RxPacket { dma_handle, len } => {
                        log(&alloc::format!("AetherNet: Received RxPacket from net-bridge for handle: {}, len: {}", dma_handle, len));
                        // Enqueue the received packet handle into the device for smoltcp to consume
                        device.enqueue_rx_packet(dma_handle, len);
                    },
                    NetPacketMsg::TxPacketAck => {
                        log("AetherNet: Received TxPacketAck from net-bridge.");
                        // Handle TX acknowledgment if needed (e.g., update internal state)
                    },
                    _ => log("AetherNet: Received unexpected NetPacketMsg from net-bridge."),
                }
            } else {
                log("AetherNet: Failed to deserialize NetPacketMsg from net-bridge.");
            }
        }

        // 1. Poll smoltcp interface for network events (e.g., ARP, ICMP, TCP/UDP activity)
        // This call will trigger device.receive() and device.transmit() internally
        iface.poll(timestamp, &mut device, &mut sockets);

        // 2. Process incoming requests from other V-Nodes (Socket API) -- on own_chan
        if let Ok(Some(req_data)) = own_chan.recv_non_blocking() { // Check for messages from other V-Nodes
            if let Ok(request) = postcard::from_bytes::<NetStackRequest>(&req_data) {
                log("AetherNet: Received request from another V-Node.");
                let response = match request {
                    NetStackRequest::OpenSocket(sock_type, local_port) => {
                        let handle = next_socket_handle;
                        next_socket_handle += 1;

                        let smoltcp_socket = match sock_type {
                            0 => { // TCP
                                log(&alloc::format!("AetherNet: Opening TCP socket on port {}", local_port));
                                let mut socket = TcpSocket::new(
                                    smoltcp::socket::TcpSocketBuffer::new(Vec::new()),
                                    smoltcp::socket::TcpSocketBuffer::new(Vec::new()),
                                );
                                if local_port != 0 { socket.listen(local_port).unwrap(); }
                                socket
                            },
                            1 => { // UDP
                                log(&alloc::format!("AetherNet: Opening UDP socket on port {}", local_port));
                                let mut socket = UdpSocket::new(
                                    smoltcp::socket::UdpSocketBuffer::new(Vec::new()),
                                    smoltcp::socket::UdpSocketBuffer::new(Vec::new()),
                                );
                                if local_port != 0 { socket.bind(local_port).unwrap(); }
                                socket
                            },
                            _ => {
                                log(&alloc::format!("AetherNet: Invalid socket type {}", sock_type));
                                return NetStackResponse::Error(100); // Invalid socket type, cannot create socket
                            }
                        };

                        // Add socket to management
                        sockets.add(smoltcp_socket);
                        smoltcp_sockets_map.insert(handle, smoltcp::socket::SocketHandle::from(sockets.len() - 1)); // Correctly get smoltcp handle
                        NetStackResponse::SocketOpened(handle)
                    },
                    NetStackRequest::Send(handle, data) => {
                        log(&alloc::format!("AetherNet: Sending {} bytes on socket {}", data.len(), handle));
                        if let Some(smoltcp_handle) = smoltcp_sockets_map.get(&handle) {
                            if let Some(socket) = sockets.get_mut(*smoltcp_handle) {
                                match socket {
                                    smoltcp::socket::Socket::Tcp(s) => {
                                        // For simplicity, assume all data can be sent immediately
                                        s.send_slice(&data).unwrap_or(0);
                                        NetStackResponse::Success
                                    },
                                    _ => NetStackResponse::Error(102), // Not a TCP/UDP socket
                                }
                            } else { NetStackResponse::Error(103) } // Smoltcp Socket not found
                        } else { NetStackResponse::Error(103) } // Our handle not found
                    },
                    NetStackRequest::SendTo(handle, remote_ip, remote_port, data) => {
                        log(&alloc::format!("AetherNet: Sending {} bytes to {}:{} on UDP socket {}", data.len(), Ipv4Address::from_bytes(&remote_ip), remote_port, handle));
                        if let Some(smoltcp_handle) = smoltcp_sockets_map.get(&handle) {
                            if let Some(socket) = sockets.get_mut(*smoltcp_handle) {
                                match socket {
                                    smoltcp::socket::Socket::Udp(s) => {
                                        s.send_slice(data.as_slice(), smoltcp::wire::IpEndpoint::new(IpAddress::v4(remote_ip[0], remote_ip[1], remote_ip[2], remote_ip[3]), remote_port)).unwrap_or(0); // Assuming send_slice handles Endpoint
                                        NetStackResponse::Success
                                    },
                                    _ => NetStackResponse::Error(102), // Not a UDP socket
                                }
                            } else { NetStackResponse::Error(103) } // Smoltcp Socket not found
                        } else { NetStackResponse::Error(103) } // Our handle not found
                    },
                    NetStackRequest::Recv(handle) => {
                        log(&alloc::format!("AetherNet: Receiving on socket {}", handle));
                        if let Some(smoltcp_handle) = smoltcp_sockets_map.get(&handle) {
                             if let Some(socket) = sockets.get_mut(*smoltcp_handle) {
                                match socket {
                                    smoltcp::socket::Socket::Tcp(s) => {
                                        let mut buffer = Vec::new(); // Or use a pre-allocated buffer
                                        if let Ok(size) = s.recv_slice(&mut buffer) {
                                            buffer.resize(size, 0);
                                            NetStackResponse::Data(buffer)
                                        }
                                         else {
                                            NetStackResponse::Data(Vec::new()) // No data
                                        }
                                    },
                                    smoltcp::socket::Socket::Udp(s) => {
                                        let mut buffer = Vec::new();
                                        if let Ok((size, _endpoint)) = s.recv_slice(&mut buffer) {
                                            buffer.resize(size, 0);
                                            NetStackResponse::Data(buffer)
                                        }
                                         else {
                                            NetStackResponse::Data(Vec::new())
                                        }
                                    },
                                    _ => NetStackResponse::Error(102), // Not a TCP/UDP socket
                                }
                            } else { NetStackResponse::Error(103) } // Smoltcp Socket not found
                        } else { NetStackResponse::Error(103) } // Our handle not found
                    },
                    NetStackRequest::CloseSocket(handle) => {
                        log(&alloc::format!("AetherNet: Closing socket {}", handle));
                        if let Some(smoltcp_handle) = smoltcp_sockets_map.remove(&handle) {
                            sockets.remove(*smoltcp_handle); // Remove from smoltcp SocketSet
                            NetStackResponse::Success
                        }
                        else {
                            NetStackResponse::Error(103) // Socket not found
                        }
                    },
                };
                own_chan.send(&response).unwrap_or_else(|_| log("AetherNet: Failed to send response."));
            } else {
                log("AetherNet: Failed to deserialize NetStackRequest.");
            }
        }
    }
}

#[panic_handler]
pub extern "C" fn panic(_info: &PanicInfo) -> ! {
    log("AetherNet Service V-Node panicked!");
    loop {}
}
