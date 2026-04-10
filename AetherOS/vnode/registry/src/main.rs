#![no_std]
#![no_main]
#![feature(alloc_error_handler)]

#[cfg(feature = "alloc")]
extern crate alloc;

#[cfg(feature = "serde")]
extern crate serde;

use alloc::format;
use alloc::string::String;
use alloc::vec::Vec;
use core::panic::PanicInfo;
use linked_list_allocator::LockedHeap;

use aetheros_common::arp_dht::{DhtValue, InMemoryDht, NodeId, PeerInfo};
use aetheros_common::examples;
use aetheros_common::ipc::vnode::VNodeChannel;
use aetheros_common::swarm_engine::global_search::{GlobalSearchService, SearchRequest};
use aetheros_common::swarm_engine::{SwarmEngine, SwarmError, SwarmTransport};
use aetheros_common::syscall::{syscall3, SYS_LOG, SUCCESS};
use aetheros_common::trust::{Aid, TrustStore};

const VNODE_HEAP_SIZE: usize = 64 * 1024;
static mut VNODE_HEAP: [u8; VNODE_HEAP_SIZE] = [0; VNODE_HEAP_SIZE];

#[global_allocator]
static ALLOCATOR: LockedHeap = LockedHeap::empty();

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

struct NoopTransport;

impl SwarmTransport for NoopTransport {
    fn fetch_chunk_from_peer(&self, _peer: &PeerInfo, _cid: [u8; 32]) -> Result<Vec<u8>, SwarmError> {
        Err(SwarmError::RoutingNotFound)
    }
}

fn main() -> ! {
    let trust_store = TrustStore::new();
    let dht_for_init = InMemoryDht::new();

    let (manifest, _chunks) = examples::hello_package::make_hello_package();
    dht_for_init.store(manifest.root_cid, DhtValue::Manifest(manifest.clone()));

    let _aid = Aid([0xBB; 32]);
    let _node_id = NodeId([0xAA; 32]);
    let local_peer = PeerInfo {
        ip_address: [127, 0, 0, 1],
        port: 7777,
        vnode_id: 1,
    };

    let swarm_engine = SwarmEngine::new(NoopTransport);
    let _ = swarm_engine.fetch_chunk_from_peer(&local_peer, manifest.root_cid);

    let search_service = GlobalSearchService::new();
    let search_request = SearchRequest::new("hello");
    let _selected = search_service.select_peers(core::slice::from_ref(&local_peer), &search_request);

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
