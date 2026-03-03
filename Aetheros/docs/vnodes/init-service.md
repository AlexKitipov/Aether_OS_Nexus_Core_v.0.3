# Init Service V-Node

## Overview

The `init-service` V-Node is a critical system component in AetherOS, responsible for managing the lifecycle of other V-Nodes. It acts as a supervisor, allowing privileged users or system components to start, stop, restart, and query the status of other services. It conceptualizes reading service configurations and interacting with a `kernel-vnode-manager`.

## Core Responsibilities

*   **V-Node Lifecycle Management**: Provides IPC endpoints to initiate, terminate, and restart other V-Nodes (services).
*   **Status Reporting**: Allows querying the current operational status of managed V-Nodes.
*   **Configuration Management**: Conceptually reads service definitions and their required capabilities from system configuration files (e.g., `/etc/services`).
*   **Resource Monitoring (Conceptual)**: In a full implementation, it would monitor the resource usage and health of running V-Nodes.

## Capabilities and Dependencies

To perform its functions, the `init-service` V-Node requires specific capabilities:

*   `CAP_IPC_ACCEPT`: To accept control requests (start, stop, status) from other privileged V-Nodes or command-line interfaces.
*   `CAP_IPC_CONNECT: "svc://aetherfs"`: To read system configuration files, such as `/etc/services`, which define known V-Nodes and their properties.
*   `CAP_IPC_CONNECT: "svc://kernel-vnode-manager"`: This is a conceptual IPC connection to a kernel-level V-Node manager, which would handle the actual low-level starting and stopping of V-Node processes and their resource allocation. (In the current stub, this is simulated).
*   `CAP_LOG_WRITE`: For logging service status changes, errors during V-Node operations, and audit trails.
*   `CAP_TIME_READ`: Potentially for scheduling periodic checks or implementing timeouts for V-Node startups/shutdowns.

## Operational Flow (High-Level)

1.  **Initialization**:
    *   Loads V-Node service configurations from `/etc/services` (simulated).
    *   Initializes its internal state to track running V-Nodes.
2.  **Request Handling**:
    *   Receives `InitRequest` messages (e.g., `ServiceStart`, `ServiceStatus`, `ServiceRestart`, `ServiceStop`) from client V-Nodes.
    *   For `ServiceStart`, it checks if the V-Node is already running and, if not, conceptually instructs the `kernel-vnode-manager` to launch it, assigning a dummy PID.
    *   For `ServiceStatus`, it returns the current running state and PID of the requested V-Node from its internal records.
    *   `ServiceRestart` simulates a stop followed by a start.
    *   `ServiceStop` simulates stopping a running V-Node.
    *   Responses (`InitResponse::Success`, `InitResponse::Status`, `InitResponse::Error`) are sent back to the client.
3.  **Event Loop**: Continuously polls its client IPC channel for new requests and processes them. Uses `SYS_TIME` to yield control to the kernel, allowing other V-Nodes to run.

## Example `vnode.yml` Configuration

```yaml
# vnode/init-service/vnode.yml
vnode:
  name: "init-service"
  version: "0.1.0"
  maintainer: "aetheros-core-team@aetheros.org"
  mode: strict # A critical system service for managing other V-Nodes

runtime:
  entrypoint: "bin/init-service.vnode"
  required_mem_mb: 16 # For service configurations, state management, and IPC buffers
  max_cpu_share: 0.05 # Primarily monitors and dispatches, low CPU usage expected

capabilities:
  - CAP_IPC_ACCEPT # To accept control requests from privileged V-Nodes/users
  - CAP_IPC_CONNECT: "svc://aetherfs" # To read /etc/services
  - CAP_IPC_CONNECT: "svc://kernel-vnode-manager" # To start/stop/monitor other V-Nodes (conceptual kernel IPC)
  - CAP_LOG_WRITE # For logging service status and events
  - CAP_TIME_READ # For scheduling or timeout mechanisms

storage:
  mounts:
    - path: "/etc/services"
      source: "aetherfs://system-config/services"
      options: [ "ro" ] # Read-only access to service definitions
    - path: "/var/run/init-service"
      source: "volatile://ramdisk"
      size: "1MB" # For temporary state like running V-Node PIDs/handles

observability:
  metrics: ["services_total", "services_running", "services_stopped", "service_starts_total", "service_restarts_total"]
```
