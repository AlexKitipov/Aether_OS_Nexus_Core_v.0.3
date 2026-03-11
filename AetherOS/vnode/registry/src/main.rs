#![no_std]
#![no_main]

extern crate alloc;

use alloc::format;
use alloc::string::String;
use alloc::vec::Vec;
use core::panic::PanicInfo;
use linked_list_allocator::LockedHeap;

use aetheros_common::arp_dht::{DhtValue, InMemoryDht, NodeId, PeerInfo};
use aetheros_common::examples;
use aetheros_common::ipc::vnode::VNodeChannel;
use aetheros_common::swarm_engine::global_search::{GlobalSearchService, SearchRequest};
use aetheros_common::swarm_engine::{SwarmEngine, SwarmTransport};
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

struct RegistryVNode {
    client_chan: VNodeChannel,
}

impl RegistryVNode {
    fn new(client_chan: VNodeChannel) -> Self {
        Self { client_chan }
    }

    fn tick(&mut self) {
        if let Ok(Some(req_data)) = self.client_chan.recv_non_blocking() {
            let data: Vec<u8> = req_data.to_vec();
            let msg = format!("Registry received {} bytes", data.len());
            log(&msg);
        }
    }
}

fn main() -> ! {
    let trust_store = TrustStore::new();
    let dht_for_init = InMemoryDht::new();

    let (manifest, _chunks) = examples::hello_package::make_hello_package();
    dht_for_init.store(manifest.root_cid, DhtValue::Manifest(manifest.clone()));

    let _aid = Aid([0xBB; 32]);
    let _node_id = NodeId([0xAA; 32]);
    let _peer_info = PeerInfo;
    let _swarm_engine = SwarmEngine;
    let _swarm_transport = SwarmTransport;
    let _search_service = GlobalSearchService;
    let _search_request = SearchRequest;

    let startup_status: String = format!(
        "Registry V-Node initialized (syscall SUCCESS = {}, trust_store = {:p})",
        SUCCESS,
        &trust_store
    );
    log(&startup_status);

    let mut registry = RegistryVNode::new(VNodeChannel::new(1));
    loop {
        registry.tick();
    }
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
