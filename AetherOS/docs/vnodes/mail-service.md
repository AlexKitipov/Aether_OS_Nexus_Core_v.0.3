# Mail Service V-Node

## Overview

The `mail-service` V-Node is responsible for managing email communication within AetherOS. It provides a standardized IPC interface for client V-Nodes (e.g., mail client applications) to send, list, and read mail messages. It leverages other system V-Nodes like `vfs` for local storage, `socket-api` for network communication, and `dns-resolver` for mail server lookups.

## Core Responsibilities

*   **Send Mail**: Allows client V-Nodes to compose and send email messages to recipients, conceptually handling interactions with mail servers (SMTP).
*   **Mailbox Management**: Provides functionality to list available mailboxes (e.g., Inbox, Sent) for the current user.
*   **Read Mail**: Enables reading specific mail messages from a designated mailbox.
*   **Local Mail Storage**: Conceptually interacts with the `vfs` V-Node to store and retrieve mail messages and mailbox structures in the user's home directory (e.g., `/home/<AID>/mail`).
*   **Network Mail Protocols**: (Conceptual) Utilizes `socket-api` to establish network connections for protocols like SMTP (Simple Mail Transfer Protocol), POP3 (Post Office Protocol 3), and IMAP (Internet Message Access Protocol).
*   **DNS Resolution**: Uses `dns-resolver` to find the IP addresses of mail servers based on hostnames.

## Capabilities and Dependencies

To perform its functions, the `mail-service` V-Node requires specific capabilities:

*   `CAP_IPC_ACCEPT`: To accept mail-related requests from client V-Nodes (e.g., `AetherMail` application).
*   `CAP_IPC_CONNECT: "svc://vfs"`: To access user-specific mail data (e.g., `/home/<AID>/mail`) for storing messages and mailbox configurations.
*   `CAP_IPC_CONNECT: "svc://socket-api"`: To perform network operations required for sending and receiving emails.
*   `CAP_IPC_CONNECT: "svc://dns-resolver"`: To resolve hostnames of mail servers.
*   `CAP_LOG_WRITE`: For logging mail operations, delivery status, and potential errors.
*   `CAP_TIME_READ`: For timestamping messages, managing connection timeouts, or periodic checks for new mail.

## Operational Flow (High-Level)

1.  **Initialization**: Establishes its IPC channels with clients, `vfs`, `socket-api`, and `dns-resolver`. Conceptually initializes user mailboxes.
2.  **Request Handling**:
    *   **`MailRequest::SendMail`**: Receives a request to send an email. Conceptually, it would resolve the recipient's mail server via `dns-resolver`, open a connection via `socket-api`, and send the email using appropriate protocols (e.g., SMTP commands). A copy is stored in the local 'Sent' mailbox via `vfs`.
    *   **`MailRequest::ListMailboxes`**: Returns a list of available mailboxes, potentially by querying `vfs` for directory names under the user's mail folder.
    *   **`MailRequest::ReadMessage`**: Retrieves a specific message from a mailbox by reading its content from `vfs`.
    *   Responses (`MailResponse::Success`, `MailResponse::Mailboxes`, `MailResponse::Message`, `MailResponse::Error`) are sent back to the client.
3.  **Background Tasks (Conceptual)**: Periodically checks for new incoming mail by connecting to mail servers (POP3/IMAP) via `socket-api` and `dns-resolver`.
4.  **Event Loop**: Continuously polls its client IPC channel for new requests and processes them. Uses `SYS_TIME` to yield control to the kernel.

## Example `vnode.yml` Configuration

```yaml
# vnode/mail-service/vnode.yml
vnode:
  name: "mail-service"
  version: "0.1.0"
  maintainer: "aetheros-core-team@aetheros.org"
  mode: strict # A core system service for messaging

runtime:
  entrypoint: "bin/mail-service.vnode"
  required_mem_mb: 16 # For message queues, temporary storage, and IPC buffers
  max_cpu_share: 0.05 # Primarily handles messages, low CPU usage expected

capabilities:
  - CAP_IPC_ACCEPT # To accept requests from client V-Nodes (e.g., mail client app)
  - CAP_IPC_CONNECT: "svc://vfs" # To access user's mailbox files (e.g., /home/<AID>/mail)
  - CAP_IPC_CONNECT: "svc://socket-api" # For sending/receiving mail via network protocols (SMTP, POP3, IMAP)
  - CAP_IPC_CONNECT: "svc://dns-resolver" # For resolving mail server hostnames
  - CAP_LOG_WRITE # For logging mail operations and errors
  - CAP_TIME_READ # For timestamping messages or internal timing

storage:
  mounts:
    - path: "/home/<AID>/mail"
      source: "aetherfs://user/<AID>/mail"
      options: [ "rw", "recursive" ] # Read/write access to user's mail directories
    - path: "/etc/mail"
      source: "aetherfs://system-config/mail"
      options: [ "ro" ] # Read-only for system-wide mail configurations

observability:
  metrics: ["mail_sent_total", "mail_received_total", "mailboxes_listed_total", "messages_read_total", "errors_total"]
```
