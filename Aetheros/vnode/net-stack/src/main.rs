#![no_std]
#![no_main]

extern crate alloc;

use core::panic::PanicInfo;
use alloc::vec::Vec;
use alloc::collections::BTreeMap;
use alloc::format;

use smoltcp::iface::{Config, Interface, SocketSet, QueryInterface};
use smoltcp::phy::Checksum;
use smoltcp::socket::{TcpSocket, UdpSocket};
use smoltcp::wire::{EthernetAddress, HardwareAddress, IpAddress, IpCidr, Ipv4Address, ETHERNET_MTU};
use smoltcp::time::Instant;

use crate::ipc::vnode::VNodeChannel;
use crate::syscall::{syscall3, SYS_LOG, SUCCESS, E_ERROR, SYS_TIME};
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

// Get current time from kernel (assuming 1 tick = 10 ms for demo)
fn get_current_time_ms() -> u64 {
    unsafe { syscall3(SYS_TIME, 0, 0, 0) * 10 }
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
    let mut sockets_storage_tcp = [None; 8]; // Example: 8 TCP sockets
    let mut sockets_storage_udp = [None; 8]; // Example: 8 UDP sockets
    let mut sockets = SocketSet::new(sockets_storage_tcp.iter_mut().chain(sockets_storage_udp.iter_mut()));

    // 4. Socket Management
    let mut next_socket_handle: u32 = 1;
    let mut smoltcp_sockets_map: BTreeMap<u32, smoltcp::socket::SocketHandle> = BTreeMap::new(); // Maps our handle to smoltcp's

    // Main event loop for the network stack
    loop {
        let timestamp = Instant::from_millis(get_current_time_ms());

        // --- Handle Incoming Messages from net-bridge V-Node via IPC --- (from net-bridge to aethernet_device)
        if let Ok(Some(net_msg_data)) = bridge_data_chan.recv_non_blocking() {
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
                    _ => log(&alloc::format!("AetherNet: Received unexpected NetPacketMsg from net-bridge: {:?}", net_packet_msg)),
                }
            } else {
                log("AetherNet: Failed to deserialize NetPacketMsg from net-bridge.");
            }
        }

        // 1. Poll smoltcp interface for network events (e.g., ARP, ICMP, TCP/UDP activity)
        // This call will trigger device.receive() and device.transmit() internally
        iface.poll(timestamp, &mut device, &mut sockets);

        // 2. Process incoming requests from other V-Nodes (Socket API) -- on own_chan
        if let Ok(Some(req_data)) = own_chan.recv_non_blocking() {
            if let Ok(request) = postcard::from_bytes::<NetStackRequest>(&req_data) {
                log(&alloc::format!("AetherNet: Received request from another V-Node: {:?}", request));
                let response = match request {
                    NetStackRequest::OpenSocket(sock_type, local_port) => {
                        let handle = next_socket_handle;
                        next_socket_handle += 1;

                        let socket_to_add = match sock_type {
                            0 => { // TCP
                                log(&alloc::format!("AetherNet: Opening TCP socket on port {}", local_port));
                                let mut socket = TcpSocket::new(
                                    smoltcp::socket::TcpSocketBuffer::new(alloc::vec![0; 1024]), // Rx buffer
                                    smoltcp::socket::TcpSocketBuffer::new(alloc::vec![0; 1024]), // Tx buffer
                                );
                                if local_port != 0 { socket.listen(local_port).unwrap(); }
                                socket
                            },
                            1 => { // UDP
                                log(&alloc::format!("AetherNet: Opening UDP socket on port {}", local_port));
                                let mut socket = UdpSocket::new(
                                    smoltcp::socket::UdpSocketBuffer::new(alloc::vec![0; 1024]), // Rx buffer
                                    smoltcp::socket::UdpSocketBuffer::new(alloc::vec![0; 1024]), // Tx buffer
                                );
                                if local_port != 0 { socket.bind(local_port).unwrap(); }
                                socket
                            },
                            _ => {
                                log(&alloc::format!("AetherNet: Invalid socket type {}", sock_type));
                                NetStackResponse::Error(100) // Invalid socket type, cannot create socket
                            }
                        };

                        if let NetStackResponse::Error(_) = socket_to_add {
                            socket_to_add // Propagate error if socket creation failed
                        } else {
                            // Add socket to management
                            let smoltcp_socket_handle = sockets.add(socket_to_add.unwrap()); // Unwrap because we know it's not an Error
                            smoltcp_sockets_map.insert(handle, smoltcp_socket_handle);
                            NetStackResponse::SocketOpened(handle)
                        }
                    },
                    NetStackRequest::Send(handle, data) => {
                        log(&alloc::format!("AetherNet: Sending {} bytes on socket {}", data.len(), handle));
                        if let Some(smoltcp_handle) = smoltcp_sockets_map.get(&handle) {
                            if let Some(socket) = sockets.get_mut(*smoltcp_handle) {
                                match socket {
                                    smoltcp::socket::Socket::Tcp(s) => {
                                        if s.can_send() {
                                            s.send_slice(&data).unwrap_or(0); // Send data, ignoring partial sends for now
                                            NetStackResponse::Success
                                        } else {
                                            log(&alloc::format!("AetherNet: TCP socket {} cannot send (buffer full or not connected)", handle));
                                            NetStackResponse::Error(104) // Cannot send
                                        }
                                    },
                                    _ => {
                                        log(&alloc::format!("AetherNet: Socket {} is not a TCP socket for Send request.", handle));
                                        NetStackResponse::Error(102) // Not a TCP/UDP socket
                                    },
                                }
                            } else {
                                log(&alloc::format!("AetherNet: Smoltcp Socket not found for handle {}.", handle));
                                NetStackResponse::Error(103)
                            }
                        } else {
                            log(&alloc::format!("AetherNet: Our handle {} not found in map.", handle));
                            NetStackResponse::Error(103)
                        }
                    },
                    NetStackRequest::SendTo(handle, remote_ip, remote_port, data) => {
                        log(&alloc::format!("AetherNet: Sending {} bytes to {}.{}.{}:{}{} on UDP socket {}", data.len(), remote_ip[0], remote_ip[1], remote_ip[2], remote_ip[3], remote_port, handle));
                        if let Some(smoltcp_handle) = smoltcp_sockets_map.get(&handle) {
                            if let Some(socket) = sockets.get_mut(*smoltcp_handle) {
                                match socket {
                                    smoltcp::socket::Socket::Udp(s) => {
                                        let remote_endpoint = smoltcp::wire::IpEndpoint::new(
                                            IpAddress::v4(remote_ip[0], remote_ip[1], remote_ip[2], remote_ip[3]),
                                            remote_port
                                        );
                                        if s.can_send() {
                                            s.send_slice(data.as_slice(), remote_endpoint).unwrap_or(0);
                                            NetStackResponse::Success
                                        } else {
                                            log(&alloc::format!("AetherNet: UDP socket {} cannot send (buffer full)", handle));
                                            NetStackResponse::Error(104) // Cannot send
                                        }
                                    },
                                    _ => {
                                        log(&alloc::format!("AetherNet: Socket {} is not a UDP socket for SendTo request.", handle));
                                        NetStackResponse::Error(102) // Not a UDP socket
                                    },
                                }
                            } else {
                                log(&alloc::format!("AetherNet: Smoltcp Socket not found for handle {}.", handle));
                                NetStackResponse::Error(103)
                            }
                        } else {
                            log(&alloc::format!("AetherNet: Our handle {} not found in map.", handle));
                            NetStackResponse::Error(103)
                        }
                    },
                    NetStackRequest::Recv(handle) => {
                        log(&alloc::format!("AetherNet: Receiving on socket {}", handle));
                        if let Some(smoltcp_handle) = smoltcp_sockets_map.get(&handle) {
                             if let Some(socket) = sockets.get_mut(*smoltcp_handle) {
                                match socket {
                                    smoltcp::socket::Socket::Tcp(s) => {
                                        if s.can_recv() {
                                            let mut buffer = alloc::vec![0; s.recv_capacity()];
                                            if let Ok(size) = s.recv_slice(&mut buffer) {
                                                buffer.truncate(size);
                                                NetStackResponse::Data(buffer)
                                            } else {
                                                log(&alloc::format!("AetherNet: Failed to recv from TCP socket {} (no data or error)", handle));
                                                NetStackResponse::Data(alloc::vec![]) // No data
                                            }
                                        } else {
                                            log(&alloc::format!("AetherNet: TCP socket {} cannot recv (buffer empty or not connected)", handle));
                                            NetStackResponse::Data(alloc::vec![]) // No data
                                        }
                                    },
                                    smoltcp::socket::Socket::Udp(s) => {
                                        if s.can_recv() {
                                            let mut buffer = alloc::vec![0; s.recv_capacity()];
                                            if let Ok((size, _endpoint)) = s.recv_slice(&mut buffer) {
                                                buffer.truncate(size);
                                                NetStackResponse::Data(buffer)
                                            } else {
                                                log(&alloc::format!("AetherNet: Failed to recv from UDP socket {} (no data or error)", handle));
                                                NetStackResponse::Data(alloc::vec![])
                                            }
                                        } else {
                                            log(&alloc::format!("AetherNet: UDP socket {} cannot recv (buffer empty)", handle));
                                            NetStackResponse::Data(alloc::vec![])
                                        }
                                    },
                                    _ => {
                                        log(&alloc::format!("AetherNet: Socket {} is not a TCP/UDP socket for Recv request.", handle));
                                        NetStackResponse::Error(102) // Not a TCP/UDP socket
                                    },
                                }
                            } else {
                                log(&alloc::format!("AetherNet: Smoltcp Socket not found for handle {}.", handle));
                                NetStackResponse::Error(103)
                            }
                        } else {
                            log(&alloc::format!("AetherNet: Our handle {} not found in map.", handle));
                            NetStackResponse::Error(103)
                        }
                    },
                    NetStackRequest::CloseSocket(handle) => {
                        log(&alloc::format!("AetherNet: Closing socket {}", handle));
                        if let Some(smoltcp_handle) = smoltcp_sockets_map.remove(&handle) {
                            sockets.remove(*smoltcp_handle);
                            NetStackResponse::Success
                        }
                        else {
                            log(&alloc::format!("AetherNet: Socket {} not found for closing.", handle));
                            NetStackResponse::Error(103) // Socket not found
                        }
                    },
                };
                own_chan.send(&response).unwrap_or_else(|_| log("AetherNet: Failed to send response to client."));
            } else {
                log("AetherNet: Failed to deserialize NetStackRequest.");
            }
        }

        // Yield to other V-Nodes to prevent busy-waiting
        unsafe { syscall3(SYS_TIME, 0, 0, 0); } // Assuming 1 tick = 10ms
    }
}

#[panic_handler]
pub extern "C" fn panic(info: &PanicInfo) -> ! {
    log(&alloc::format!("AetherNet Service V-Node panicked! Info: {:?}", info));
    loop {}
}
