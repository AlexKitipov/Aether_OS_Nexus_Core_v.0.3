
# Service Manager / Init V-Node (svc://init-service)

## Overview

The `init-service` V-Node acts as the system's service manager, similar to `systemd` or `init` in traditional operating systems, but within the AetherOS microkernel architecture. Its primary responsibilities include starting, stopping, restarting, and monitoring other V-Nodes based on configuration and IPC requests. This V-Node is crucial for maintaining the overall health and lifecycle of system services.

## IPC Protocol

Communication with the `init-service` V-Node occurs via IPC, using the `InitRequest` and `InitResponse` enums defined in `src/ipc/init_ipc.rs`.

### InitRequest Enum (Client -> init-service)

Client V-Nodes or privileged user tools send these requests to `svc://init-service` to manage system services.

```rust
#[derive(Debug, Serialize, Deserialize)]
pub enum InitRequest {
    /// Start a V-Node by its name.
    ServiceStart { service_name: String },
    /// Get the status of a V-Node.
    ServiceStatus { service_name: String },
    /// Restart a V-Node.
    ServiceRestart { service_name: String },
    /// Stop a V-Node.
    ServiceStop { service_name: String },
}
```

**Parameters:**

*   `service_name`: A `String` representing the unique name of the V-Node service (e.g., "aethernet-service", "socket-api").

### InitResponse Enum (init-service -> Client)

`svc://init-service` sends these responses back to the client V-Node after processing an `InitRequest`.

```rust
#[derive(Debug, Serialize, Deserialize)]
pub enum InitResponse {
    /// Indicates successful operation.
    Success(String), // Success message
    /// Returns the status of a V-Node.
    Status { service_name: String, is_running: bool, pid: Option<u64> },
    /// Indicates an error occurred.
    Error(String), // Error message
}
```

**Return Values:**

*   `Success(String)`: Indicates a successful operation, with a descriptive message.
*   `Status { service_name: String, is_running: bool, pid: Option<u64> }`: Returns the status of the queried service. `is_running` is true if the service is active, and `pid` is the conceptual process ID/handle if available.
*   `Error(String)`: An internal error occurred or the request failed, with a descriptive message.

## Functionality

The `init-service` V-Node performs the following key functions:

1.  **Request Handling**: Listens for `InitRequest` messages on its dedicated IPC channel.
2.  **Configuration Management**: Reads service definitions and configurations from `/etc/services`. This file specifies V-Node names, entrypoints, required capabilities, and other launch parameters.
3.  **V-Node Lifecycle Management**: Conceptually interacts with the kernel's V-Node manager (e.g., via a `svc://kernel-vnode-manager` IPC channel) to:
    *   **Start V-Nodes**: Load and launch V-Nodes as per their configuration.
    *   **Stop V-Nodes**: Terminate running V-Nodes.
    *   **Restart V-Nodes**: Perform a stop-then-start sequence.
    *   **Monitor V-Nodes**: Track the running status and health of V-Nodes.
4.  **State Tracking**: Maintains an internal record of all configured and currently running V-Nodes, including their conceptual PIDs and status.
5.  **Error Handling**: Reports issues such as unknown service names, services already running, or failures during V-Node launch/termination.

## Usage Examples

### Example: Starting a Service

```rust
// Pseudocode for a privileged client V-Node or ash CLI wanting to start a service

let mut init_service_chan = VNodeChannel::new(6); // IPC Channel to svc://init-service

// Request to start the aethernet-service
let request = InitRequest::ServiceStart { service_name: String::from("aethernet-service") };
match init_service_chan.send_and_recv::<InitRequest, InitResponse>(&request) {
    Ok(InitResponse::Success(msg)) => {
        log!("Successfully started service: {}", msg);
    },
    Ok(InitResponse::Error(msg)) => {
        log!("Failed to start service: {}", msg);
    },
    _ => log!("Unexpected response from Init Service"),
}
```

### Example: Getting Service Status

```rust
// Pseudocode for a client V-Node or ash CLI wanting to check service status

let mut init_service_chan = VNodeChannel::new(6);

// Request status for the socket-api service
let request = InitRequest::ServiceStatus { service_name: String::from("socket-api") };
match init_service_chan.send_and_recv::<InitRequest, InitResponse>(&request) {
    Ok(InitResponse::Status { service_name, is_running, pid }) => {
        log!("Service {}: Running: {}, PID: {:?}", service_name, is_running, pid);
    },
    Ok(InitResponse::Error(msg)) => {
        log!("Failed to get service status: {}", msg);
    },
    _ => log!("Unexpected response from Init Service"),
}
```

This documentation outlines the critical role of the Service Manager in orchestrating the AetherOS system and how other V-Nodes or administrative tools can interact with it securely and efficiently through its well-defined IPC interface.
