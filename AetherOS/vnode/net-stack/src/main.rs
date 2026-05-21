extern crate alloc;

use linked_list_allocator::LockedHeap;
use alloc::collections::BTreeMap;

use smoltcp::iface::{InterfaceBuilder, SocketHandle};
use smoltcp::socket::TcpSocket;
use smoltcp::wire::{EthernetAddress, HardwareAddress, IpAddress, IpCidr};
use smoltcp::time::Instant;

use common::ipc::vnode::VNodeChannel;
use common::ipc::IpcSend;
use common::syscall::{syscall3, SYS_LOG, SUCCESS, SYS_TIME};
use common::ipc::net_ipc::{NetPacketMsg, NetStackRequest, NetStackResponse};

mod aethernet_device;
use aethernet_device::AetherNetDevice;

// Temporary log function for V-Nodes

const VNODE_HEAP_SIZE: usize = 64 * 1024;
static mut VNODE_HEAP: [u8; VNODE_HEAP_SIZE] = [0; VNODE_HEAP_SIZE];

#[global_allocator]
static GLOBAL_ALLOCATOR: LockedHeap = LockedHeap::empty();

fn init_allocator() {
    unsafe {
        GLOBAL_ALLOCATOR.lock().init(VNODE_HEAP.as_mut_ptr(), VNODE_HEAP_SIZE);
    }
}

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
    syscall3(SYS_TIME, 0, 0, 0) * 10
}

fn main() { _start() }

#[no_mangle]
pub extern "C" fn _start() -> ! {
    init_allocator();
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
    let mut socket_entries = alloc::vec![];
    let mut iface = InterfaceBuilder::new(device, socket_entries)
        .hardware_addr(HardwareAddress::Ethernet(ethernet_addr))
        .ip_addrs([IpCidr::new(IpAddress::v4(10, 0, 2, 15), 24)])
        .finalize();

    // Assign a static IP address
    log(&alloc::format!("AetherNet: IP Address set to {}", IpAddress::v4(10,0,2,15)));

    // 3. Initialize smoltcp SocketSet

    // 4. Socket Management
    let mut next_socket_handle: u32 = 1;
    let mut smoltcp_sockets_map: BTreeMap<u32, SocketHandle> = BTreeMap::new(); // Maps our handle to smoltcp's

    // Main event loop for the network stack
    loop {
        let timestamp = Instant::from_millis(get_current_time_ms() as i64);

        // --- Handle Incoming Messages from net-bridge V-Node via IPC --- (from net-bridge to aethernet_device)
        if let Ok(Some(net_msg_data)) = bridge_data_chan.recv_non_blocking() {
            if let Ok(net_packet_msg) = postcard::from_bytes::<NetPacketMsg>(&net_msg_data) {
                match net_packet_msg {
                    NetPacketMsg::RxPacket { dma_handle, len } => {
                        log(&alloc::format!("AetherNet: Received RxPacket from net-bridge for handle: {}, len: {}", dma_handle, len));
                        // Enqueue the received packet handle into the device for smoltcp to consume
                        iface.device_mut().enqueue_rx_packet(dma_handle, len);
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
        let _ = iface.poll(timestamp);

        // 2. Process incoming requests from other V-Nodes (Socket API) -- on own_chan
        if let Ok(Some(req_data)) = own_chan.recv_non_blocking() {
            if let Ok(request) = postcard::from_bytes::<NetStackRequest>(&req_data) {
                log(&alloc::format!("AetherNet: Received request from another V-Node: {:?}", request));
                let response = match request {
                    NetStackRequest::OpenSocket(sock_type, local_port) => {
                        let handle = next_socket_handle;
                        next_socket_handle += 1;

                        match sock_type {
                            0 => { // TCP
                                log(&alloc::format!("AetherNet: Opening TCP socket on port {}", local_port));
                                let mut socket = TcpSocket::new(smoltcp::socket::TcpSocketBuffer::new(vec![0; 1024]), smoltcp::socket::TcpSocketBuffer::new(vec![0; 1024]));
                                if local_port != 0 { socket.listen(local_port).unwrap(); }
                                let smoltcp_socket_handle = iface.add_socket(socket);
                                smoltcp_sockets_map.insert(handle, smoltcp_socket_handle);
                                NetStackResponse::SocketOpened(handle)
                            },
                            1 => NetStackResponse::Error(105),
                            _ => {
                                log(&alloc::format!("AetherNet: Invalid socket type {}", sock_type));
                                NetStackResponse::Error(100) // Invalid socket type, cannot create socket
                            }
                        }
                    },
                    NetStackRequest::Send(handle, data) => {
                        log(&alloc::format!("AetherNet: Sending {} bytes on socket {}", data.len(), handle));
                        if let Some(smoltcp_handle) = smoltcp_sockets_map.get(&handle) {
                            {
                                let s = iface.get_socket::<TcpSocket>(*smoltcp_handle);
                                        if s.can_send() {
                                            s.send_slice(&data).unwrap_or(0); // Send data, ignoring partial sends for now
                                            NetStackResponse::Success
                                        } else {
                                            log(&alloc::format!("AetherNet: TCP socket {} cannot send (buffer full or not connected)", handle));
                                            NetStackResponse::Error(104) // Cannot send
                                        }
                            }
                        } else {
                            log(&alloc::format!("AetherNet: Our handle {} not found in map.", handle));
                            NetStackResponse::Error(103)
                        }
                    },
                    NetStackRequest::SendTo(handle, remote_ip, remote_port, data) => {
                        let _ = (remote_ip, remote_port, data);
                        NetStackResponse::Error(105)
                    },
                    NetStackRequest::Recv(handle) => {
                        log(&alloc::format!("AetherNet: Receiving on socket {}", handle));
                        if let Some(smoltcp_handle) = smoltcp_sockets_map.get(&handle) {
                             {
                                let s = iface.get_socket::<TcpSocket>(*smoltcp_handle);
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
                            }
                        } else {
                            log(&alloc::format!("AetherNet: Our handle {} not found in map.", handle));
                            NetStackResponse::Error(103)
                        }
                    },
                    NetStackRequest::CloseSocket(handle) => {
                        log(&alloc::format!("AetherNet: Closing socket {}", handle));
                        if let Some(smoltcp_handle) = smoltcp_sockets_map.remove(&handle) {
                            iface.remove_socket(smoltcp_handle);
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
