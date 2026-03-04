#![no_std]
#![no_main]

extern crate alloc;

use core::panic::PanicInfo;
use alloc::vec::Vec;
use alloc::collections::BTreeMap;
use alloc::format;

use crate::ipc::vnode::VNodeChannel;
use crate::syscall::{syscall3, SYS_LOG, SUCCESS};
// RegistryService is a placeholder for future, more complex registry logic.
// use crate::registry_service::RegistryService;
use crate::swarm_engine::{SwarmEngine, SwarmTransport};
use crate::arp_dht::{InMemoryDht, PeerInfo, NodeId};
use crate::trust::{TrustStore, Aid};

// Import NexusNetTransport - our concrete implementation of SwarmTransport using libnexus-net
use crate::swarm_engine::nexus_net_transport::NexusNetTransport;
// Import GlobalSearchService for demonstrating search capabilities
use crate::swarm_engine::global_search::GlobalSearchService;

// Temporary log function for V-Nodes. This sends a syscall to the kernel for logging.
fn log(msg: &str) {
    unsafe {
        let res = syscall3(
            SYS_LOG,
            msg.as_ptr() as u64,
            msg.len() as u64,
            0 // arg3 is unused for SYS_LOG
        );
        if res != SUCCESS { /* Log error, or maybe just ignore for a logging utility */ }
    }
}

#[no_mangle]
pub extern "C" fn _start() -> ! {
    // The Registry V-Node's dedicated IPC channel for receiving requests.
    // Assuming channel ID 1 is reserved for the Registry service.
    let mut own_chan = VNodeChannel::new(1);

    log("Registry V-Node starting up...");

    // 1. Initialize NexusNetTransport (which internally uses libnexus-net and talks to svc://aethernet).
    // This is crucial for the SwarmEngine to perform network operations.
    let transport = match NexusNetTransport::new() {
        Ok(t) => {
            log("Registry: NexusNetTransport initialized successfully.");
            t
        },
        Err(e) => {
            // If NexusNetTransport fails to initialize, the Registry cannot function.
            // It's a critical error, so we panic.
            log(&alloc::format!("Registry: Failed to initialize NexusNetTransport: {:?}. Panicking.", e));
            panic!("NexusNetTransport initialization failed");
        }
    };

    // --- Swarm Engine Initialization ---
    // These are dummy values for demonstration. In a real system, AID and NodeId
    // would be derived from user identity and system configuration.
    let trust_store = TrustStore::new();
    let local_aid = Aid([0xCD; 32]); // Dummy local AID
    let local_node_id = NodeId([0; 32]); // Dummy NodeId for local DHT

    // Initialize an in-memory DHT for local testing. This would eventually be persistent.
    let mut dht_for_init = InMemoryDht::new(local_node_id.clone());

    // Add some dummy peers to simulate a network presence for the DHT.
    dht_for_init.add_peer(PeerInfo {
        id: NodeId([0xAA; 32]),
        aid: crate::trust::Aid([0xBB; 32]),
        ip_address: [10, 0, 2, 1], // Example peer IP (could be QEMU host or another V-Node)
        port: 60000, // Example peer port for swarm communication
    });

    // Load a dummy package manifest for demonstration purposes. This package's CID
    // can be 'looked up' and 'fetched' by the SwarmEngine.
    let (manifest, _chunks) = crate::examples::hello_package::make_hello_package();
    dht_for_init.store(manifest.root_cid, crate::arp_dht::DhtValue::Manifest(manifest.clone()));

    // Instantiate GlobalSearchService and SwarmEngine with the initialized components.
    let global_search_service = GlobalSearchService::new(dht_for_init.clone(), trust_store.clone(), local_aid.clone());
    let mut swarm = SwarmEngine::new(transport, dht_for_init, trust_store.clone(), local_aid.clone());
    log("Registry: SwarmEngine and GlobalSearchService initialized.");
    // --- End Swarm Engine Initialization ---

    // --- Demonstration of Swarm Engine Functionality ---

    // Simulate fetching a package from the swarm using the initialized network transport.
    // This demonstrates the core capability of the Registry: retrieving `.ax` packages.
    log(&alloc::format!("Registry: Attempting to fetch dummy package '{}' (CID: {:?})...", manifest.name, manifest.root_cid.as_bytes()));
    match swarm.fetch_package(&manifest) {
        Ok(data) => {
            log(&alloc::format!("Registry: Successfully fetched package '{}' ({} bytes).", manifest.name, data.len()));
            // In a real scenario, 'data' would be processed, verified, and stored locally.
        },
        Err(e) => {
            log(&alloc::format!("Registry: Failed to fetch package '{}': {:?}.", manifest.name, e));
        }
    }

    // Demonstrate Global Search capability - looking up packages by keywords.
    let search_request = crate::swarm_engine::global_search::SearchRequest::KeywordSearch { query: alloc::string::String::from("hello") };
    log(&alloc::format!("Registry: Performing Global Search for keyword: '{}'.", "hello"));
    let search_response = global_search_service.handle_search_request(search_request);
    log(&alloc::format!("Registry: Global Search Response: {:?}", search_response));

    // --- Main Event Loop ---
    loop {
        // The Registry V-Node would typically be idling here, waiting for IPC requests
        // from other V-Nodes (e.g., AetherShell requesting a package install, or
        // the kernel notifying of a new network event relevant to swarm discovery).
        log("Registry V-Node idling, waiting for IPC requests...");
        
        // This call blocks the V-Node until an IPC message arrives on its channel (ID 1).
        // This prevents busy-waiting and allows the kernel to schedule other V-Nodes.
        let _ = own_chan.recv_blocking();

        // In a more advanced implementation, the loop might also periodically trigger
        // background swarm maintenance tasks (e.g., DHT refreshes, peer discovery).
    }
}

#[panic_handler]
pub extern "C" fn panic(info: &PanicInfo) -> ! {
    // When the Registry V-Node panics, log the panic information.
    log(&alloc::format!("Registry V-Node panicked! Info: {:?}", info));
    // In a production system, this might trigger a system-wide error handler or reboot.
    // For now, it enters an infinite loop to prevent further execution.
    loop {}
}
