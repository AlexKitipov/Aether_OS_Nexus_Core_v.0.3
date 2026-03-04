
# Shell V-Node (svc://shell)

## Overview

The `shell` V-Node serves as the primary command-line interpreter for AetherOS, analogous to `bash` or `zsh` in traditional operating systems. It provides an interactive interface for users and other V-Nodes to execute commands, manage files, interact with system services, and perform network lookups. Designed with the microkernel philosophy, the `shell` V-Node itself is a relatively thin layer, delegating complex operations to specialized V-Nodes like VFS, Init Service, and DNS Resolver via IPC.

## IPC Protocol

Communication with the `shell` V-Node occurs via IPC, using the `ShellRequest` and `ShellResponse` enums defined in `src/ipc/shell_ipc.rs`.

### ShellRequest Enum (Client -> shell)

Client V-Nodes (e.g., `AetherTerminal`) send these requests to `svc://shell` to execute commands or query its state.

```rust
#[derive(Debug, Serialize, Deserialize)]
pub enum ShellRequest {
    /// Request to execute a command with its arguments.
    ExecuteCommand { command: String, args: Vec<String> },
    /// Request to change the current working directory.
    ChangeDirectory { path: String },
    /// Request to get the current working directory.
    GetCurrentDirectory,
}
```

**Parameters:**

*   `command`: A `String` representing the name of the command to execute (e.g., "ls", "cd", "ping", "start").
*   `args`: A `Vec<String>` containing the arguments for the command.
*   `path`: A `String` representing the target path for directory operations.

### ShellResponse Enum (shell -> Client)

`svc://shell` sends these responses back to the client V-Node after processing a `ShellRequest`.

```rust
#[derive(Debug, Serialize, Deserialize)]
pub enum ShellResponse {
    /// Successful execution of a command, with its output and exit code.
    CommandOutput { stdout: String, stderr: String, exit_code: i32 },
    /// Indicates a successful operation without specific output.
    Success(String),
    /// Returns the current working directory.
    CurrentDirectory(String),
    /// Indicates an error occurred during the operation.
    Error(String),
}
```

**Return Values:**

*   `CommandOutput { stdout: String, stderr: String, exit_code: i32 }`: Returns the standard output, standard error, and exit code from command execution.
*   `Success(String)`: Indicates a successful operation, with an optional descriptive message.
*   `CurrentDirectory(String)`: Returns the shell's current working directory.
*   `Error(String)`: An internal error occurred or the request failed, with a descriptive message.

## Functionality

The `shell` V-Node provides the following core functionalities:

1.  **Command Execution**: Parses and executes commands received via `ExecuteCommand` requests.
2.  **Built-in Commands**: Implements basic shell commands directly:
    *   `cd <path>`: Changes the current working directory. It interacts with the `svc://vfs` (Virtual File System) to validate paths.
    *   `ls`: Lists the contents of the current directory. It queries `svc://vfs` for directory entries.
    *   `ping <hostname>`: Performs a network reachability test. It leverages `svc://dns-resolver` to resolve hostnames to IP addresses.
    *   `start <service_name>`: Initiates the startup of another V-Node. It sends requests to `svc://init-service`.
3.  **V-Node Interaction**: Communicates with other essential system V-Nodes via IPC to fulfill command requests:
    *   **`svc://vfs`**: For all filesystem-related operations (reading directory contents, changing directories).
    *   **`svc://init-service`**: For managing the lifecycle of other V-Nodes (starting, stopping, restarting services).
    *   **`svc://dns-resolver`**: For resolving hostnames to IP addresses, critical for network-related commands.
4.  **Current Working Directory Management**: Tracks and updates the shell's `current_dir` based on `cd` commands.
5.  **Command History**: Maintains a history of executed commands.

## Usage Examples

### Example 1: Executing `ls` (List Directory Contents)

```rust
// Pseudocode for client V-Node (e.g., AetherTerminal) sending an 'ls' command

let mut shell_chan = VNodeChannel::new(8); // IPC Channel to svc://shell

let request = ShellRequest::ExecuteCommand { command: String::from("ls"), args: Vec::new() };
match shell_chan.send_and_recv::<ShellRequest, ShellResponse>(&request) {
    Ok(ShellResponse::CommandOutput { stdout, stderr, exit_code }) => {
        log!("ls stdout:\n{}", stdout);
        if !stderr.is_empty() { log!("ls stderr:\n{}", stderr); }
        log!("ls exit code: {}", exit_code);
    },
    Ok(ShellResponse::Error(msg)) => {
        log!("ls command error: {}", msg);
    },
    _ => log!("Unexpected response from Shell"),
}
```

### Example 2: Changing Directory (`cd`)

```rust
// Pseudocode for client V-Node sending a 'cd' command

let mut shell_chan = VNodeChannel::new(8);

let request = ShellRequest::ChangeDirectory { path: String::from("/home/user/documents") };
match shell_chan.send_and_recv::<ShellRequest, ShellResponse>(&request) {
    Ok(ShellResponse::Success(msg)) => {
        log!("cd successful: {}", msg);
    },
    Ok(ShellResponse::Error(msg)) => {
        log!("cd command error: {}", msg);
    },
    _ => log!("Unexpected response from Shell"),
}

// Optionally, get current directory to confirm
let get_cwd_request = ShellRequest::GetCurrentDirectory;
match shell_chan.send_and_recv::<ShellRequest, ShellResponse>(&get_cwd_request) {
    Ok(ShellResponse::CurrentDirectory(cwd)) => {
        log!("Current working directory: {}", cwd);
    },
    _ => log!("Failed to get current directory"),
}
```

### Example 3: Pinging a Hostname

```rust
// Pseudocode for client V-Node sending a 'ping' command

let mut shell_chan = VNodeChannel::new(8);

let request = ShellRequest::ExecuteCommand { command: String::from("ping"), args: vec![String::from("example.com")] };
match shell_chan.send_and_recv::<ShellRequest, ShellResponse>(&request) {
    Ok(ShellResponse::CommandOutput { stdout, stderr, exit_code }) => {
        log!("ping stdout:\n{}", stdout);
        if !stderr.is_empty() { log!("ping stderr:\n{}", stderr); }
        log!("ping exit code: {}", exit_code);
    },
    Ok(ShellResponse::Error(msg)) => {
        log!("ping command error: {}", msg);
    },
    _ => log!("Unexpected response from Shell"),
}
```

This documentation highlights the `shell` V-Node's role as a user-facing gateway to the AetherOS ecosystem, demonstrating its modularity and reliance on IPC for system interaction.
