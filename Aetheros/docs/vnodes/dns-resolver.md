# DNS Resolver V-Node

## Overview

The `dns-resolver` V-Node is a critical system service responsible for resolving human-readable hostnames into IP addresses (and vice-versa) for other V-Nodes. It acts as a client to the `socket-api` V-Node to perform network DNS queries and manages a local cache of resolved entries.

## Core Responsibilities

*   **Hostname Resolution**: Provides an IPC interface for other V-Nodes to query for IP addresses associated with a given hostname.
*   **DNS Query Management**: Constructs and sends DNS query packets over UDP using the `socket-api` V-Node.
*   **Response Parsing**: Parses incoming DNS response packets to extract resolved IP addresses.
*   **DNS Caching**: Maintains a time-limited cache of recently resolved hostnames to improve performance and reduce network traffic.
*   **Configuration Reading**: Conceptually reads DNS server configurations (e.g., `/etc/network/resolv.conf`) via the `aetherfs` V-Node.

## Capabilities and Dependencies

To perform its functions, the `dns-resolver` V-Node requires specific capabilities:

*   `CAP_IPC_CONNECT: "svc://socket-api"`: To send UDP packets for DNS queries and receive responses.
*   `CAP_IPC_ACCEPT`: To accept DNS resolution requests from client V-Nodes (e.g., `shell`, `webview`, `mail-service`).
*   `CAP_IPC_CONNECT: "svc://aetherfs"`: To read network configuration files like `resolv.conf`.
*   `CAP_TIME_READ`: For managing cache entry TTLs and timeouts for DNS queries.
*   `CAP_LOG_WRITE`: For logging resolution events, cache hits/misses, and errors.

## Operational Flow (High-Level)

1.  **Initialization**:
    *   Reads DNS server configurations.
    *   Opens a UDP socket via `socket-api` to use for all outgoing DNS queries.
2.  **Request Handling**:
    *   Receives `DnsRequest::ResolveHostname` messages from client V-Nodes.
    *   Checks its internal cache for a valid, unexpired entry.
    *   If a cache hit: returns immediately with the cached IP address.
    *   If a cache miss or expired:
        *   Constructs a DNS query packet.
        *   Sends the query packet to a configured DNS server via `socket-api`'s UDP `SendTo` functionality.
        *   Waits for a response from `socket-api`.
        *   Parses the DNS response.
        *   Caches the result with a TTL.
        *   Returns `DnsResponse::ResolvedHostname` or `DnsResponse::NotFound`/`Error`.
3.  **Event Loop**: Continuously polls its client IPC channel for new requests and processes them. Uses `SYS_TIME` to yield control to the kernel, allowing other V-Nodes to run.

## Example `vnode.yml` Configuration

```yaml
# vnode/dns-resolver/vnode.yml
vnode:
  name: "dns-resolver"
  version: "0.1.0"
  maintainer: "aetheros-core-team@aetheros.org"
  mode: strict

runtime:
  entrypoint: "bin/dns-resolver.vnode"
  required_mem_mb: 8
  max_cpu_share: 0.02

capabilities:
  - CAP_IPC_CONNECT: "svc://socket-api"
  - CAP_IPC_ACCEPT
  - CAP_IPC_CONNECT: "svc://aetherfs"
  - CAP_TIME_READ
  - CAP_LOG_WRITE

storage:
  mounts:
    - path: "/etc/network/resolv.conf"
      source: "aetherfs://system-config/network/resolv.conf"
      options: [ "ro" ]

observability:
  metrics: ["dns_queries_total", "dns_resolutions_success_total", "dns_resolutions_failed_total", "dns_cache_hits_total", "dns_cache_size_bytes"]
```
