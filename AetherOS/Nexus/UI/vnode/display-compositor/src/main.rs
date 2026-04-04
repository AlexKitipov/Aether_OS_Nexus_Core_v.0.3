#![no_std]
#![no_main]
#![feature(alloc_error_handler)]

#[cfg(feature = "alloc")]
extern crate alloc;

#[cfg(feature = "serde")]
extern crate serde;

use alloc::format;
use alloc::string::String;
use alloc::vec;
use alloc::vec::Vec;
use core::panic::PanicInfo;
use linked_list_allocator::LockedHeap;

use aetheros_common::ipc::vnode::VNodeChannel;
use aetheros_common::arp_dht::PeerInfo;
use aetheros_common::swarm_engine::{SwarmEngine, SwarmError, SwarmTransport};
use aetheros_common::syscall::{syscall3, SYS_LOG, SUCCESS};
use aetheros_common::trust::{Aid, TrustStore};

const VNODE_HEAP_SIZE: usize = 64 * 1024;
static mut VNODE_HEAP: [u8; VNODE_HEAP_SIZE] = [0; VNODE_HEAP_SIZE];

#[global_allocator]
static ALLOCATOR: LockedHeap = LockedHeap::empty();

#[alloc_error_handler]
fn alloc_error_handler(_layout: core::alloc::Layout) -> ! {
    loop {}
}

fn init_allocator() {
    unsafe {
        ALLOCATOR.lock().init(VNODE_HEAP.as_mut_ptr(), VNODE_HEAP_SIZE);
    }
}

fn log(msg: &str) {
    let _ = syscall3(SYS_LOG, msg.as_ptr() as u64, msg.len() as u64, 0);
}

struct NoopTransport;

impl SwarmTransport for NoopTransport {
    fn fetch_chunk_from_peer(&self, _peer: &PeerInfo, _cid: [u8; 32]) -> Result<Vec<u8>, SwarmError> {
        Err(SwarmError::RoutingNotFound)
    }
}

fn main() -> ! {
    let _channel = VNodeChannel::new(11);
    let _trust_store = TrustStore::new();
    let _aid = Aid([0; 32]);
    let local_peer = PeerInfo {
        ip_address: [127, 0, 0, 1],
        port: 7777,
        vnode_id: 11,
    };
    let swarm_engine = SwarmEngine::new(NoopTransport);
    let _ = swarm_engine.fetch_chunk_from_peer(&local_peer, [0; 32]);

    let _buffer: Vec<u8> = vec![0, 1, 2, 3];
    let status: String = format!("display-compositor placeholder started (SUCCESS={})", SUCCESS);
    log(&status);

    loop {}
}

#[no_mangle]
pub extern "C" fn _start() -> ! {
    init_allocator();
    main()
}

#[panic_handler]
fn panic(_info: &PanicInfo) -> ! {
    loop {}
}
