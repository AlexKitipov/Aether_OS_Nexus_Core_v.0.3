
// vnode/shell/src/main.rs

#![no_std]
#![no_main]

extern crate alloc;

use core::panic::PanicInfo;
use linked_list_allocator::LockedHeap;
use alloc::vec::Vec;
use alloc::collections::BTreeMap;
use alloc::format;
use alloc::string::{String, ToString};

use common::ipc::vnode::VNodeChannel;
use common::syscall::{syscall3, SYS_LOG, SUCCESS, SYS_TIME};
use common::shell_ipc::{ShellRequest, ShellResponse, LogLevel};
use common::vfs_ipc::{VfsRequest, VfsResponse, Fd, VfsMetadata};
use common::init_ipc::{InitRequest, InitResponse};
use common::dns_ipc::{DnsRequest, DnsResponse};
use common::logger_ipc::{LoggerRequest, LoggerResponse};
use common::echo_ipc::{EchoRequest, EchoResponse};
use common::test_ipc::{TestRequest, TestResponse};
use common::channel::id::ChannelId; // Import ChannelId

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

// Placeholder for shell state
struct ShellService {
    client_chan: VNodeChannel, // Channel for AetherTerminal or other client V-Nodes
    vfs_chan: VNodeChannel, // Channel to svc://vfs
    init_chan: VNodeChannel, // Channel to svc://init-service
    dns_chan: VNodeChannel, // Channel to svc://dns-resolver
    logger_chan: VNodeChannel, // Channel to svc://logger
    echo_chan: VNodeChannel, // Channel to svc://echo
    test_chan: VNodeChannel, // Channel to svc://test

    current_dir: String,
    command_history: Vec<String>,
    // Add more state as needed, e.g., environmental variables
}

impl ShellService {
    fn new(
        client_chan_id: ChannelId,
        vfs_chan_id: ChannelId,
        init_chan_id: ChannelId,
        dns_chan_id: ChannelId,
        logger_chan_id: ChannelId,
        echo_chan_id: ChannelId,
        test_chan_id: ChannelId,
    ) -> Self {
        let client_chan = VNodeChannel::new(client_chan_id);
        let vfs_chan = VNodeChannel::new(vfs_chan_id);
        let init_chan = VNodeChannel::new(init_chan_id);
        let dns_chan = VNodeChannel::new(dns_chan_id);
        let logger_chan = VNodeChannel::new(logger_chan_id);
        let echo_chan = VNodeChannel::new(echo_chan_id);
        let test_chan = VNodeChannel::new(test_chan_id);

        log("Shell Service: Initializing...");

        Self {
            client_chan,
            vfs_chan,
            init_chan,
            dns_chan,
            logger_chan,
            echo_chan,
            test_chan,
            current_dir: String::from("/"), // Default to root
            command_history: Vec::new(),
        }
    }

    // Helper to parse a single command string into command name and arguments
    fn parse_single_command_line(line: &str) -> (String, Vec<String>) {
        let mut parts = line.split_whitespace();
        let cmd = parts.next().unwrap_or("").to_string();
        let args = parts.map(|s| s.to_string()).collect();
        (cmd, args)
    }

    // Helper to execute a single, non-piped command. This refactors the existing match statement.
    fn execute_internal_command(&mut self, command: String, args: Vec<String>) -> ShellResponse {
        log(&alloc::format!("Shell: Executing internal command: {} with args: {:?}", command, args));
        match command.as_str() {
            "cd" => {
                if let Some(path) = args.get(0) {
                    return self.handle_change_directory(path.to_string());
                } else {
                    return ShellResponse::Error("cd: missing argument".to_string());
                }
            },
            "ls" => {
                let path = args.get(0).cloned().unwrap_or_else(|| self.current_dir.clone());
                match self.vfs_chan.send_and_recv::<VfsRequest, VfsResponse>(&VfsRequest::List { path }) {
                    Ok(VfsResponse::DirectoryEntries(entries)) => {
                        let mut output = String::new();
                        for (name, _) in entries {
                            output.push_str(&name);
                            output.push_str("
");
                        }
                        ShellResponse::CommandOutput { stdout: output, stderr: String::new(), exit_code: 0 }
                    },
                    Ok(VfsResponse::Error { message, .. }) => ShellResponse::Error(format!("ls: {}", message)),
                    _ => ShellResponse::Error("ls: Unexpected response from VFS".to_string()),
                }
            },
            "ping" => {
                if let Some(hostname) = args.get(0) {
                    match self.dns_chan.send_and_recv::<DnsRequest, DnsResponse>(&DnsRequest::ResolveHostname { hostname: hostname.clone() }) {
                        Ok(DnsResponse::ResolvedHostname { ip_address, .. }) => {
                            ShellResponse::CommandOutput { stdout: format!("Pinging {} ({}.{}.{}.{})", hostname, ip_address[0], ip_address[1], ip_address[2], ip_address[3]), stderr: String::new(), exit_code: 0 }
                        },
                        Ok(DnsResponse::NotFound { query }) => ShellResponse::Error(format!("ping: Host '{}' not found.", query)),
                        Ok(DnsResponse::Error { message }) => ShellResponse::Error(format!("ping: DNS error: {}", message)),
                        _ => ShellResponse::Error("ping: Unexpected response from DNS Resolver".to_string()),
                    }
                } else {
                    ShellResponse::Error("ping: missing hostname".to_string())
                }
            },
            "start" => {
                if let Some(service_name) = args.get(0) {
                    match self.init_chan.send_and_recv::<InitRequest, InitResponse>(&InitRequest::ServiceStart { service_name: service_name.clone() }) {
                        Ok(InitResponse::Success(msg)) => ShellResponse::Success(msg),
                        Ok(InitResponse::Error(msg)) => ShellResponse::Error(format!("start: {}", msg)),
                        _ => ShellResponse::Error("start: Unexpected response from Init Service".to_string()),
                    }
                } else {
                    ShellResponse::Error("start: missing service name".to_string())
                }
            },
            "logger" => { // This will now be handled by RunLoggerCommand variant directly
                return ShellResponse::Error("logger: Please use 'log' command with specific options (e.g., 'log info "message"')".to_string());
            },
            "echo" => { // This will now be handled by RunEchoCommand variant directly
                return ShellResponse::Error("echo: Please use 'echo' command (e.g., 'echo "hello"')".to_string());
            },
            "test" => { // This will now be handled by RunTestCommand variant directly
                return ShellResponse::Error("test: Please use 'test' command with specific options (e.g., 'test echo "message"')".to_string());
            },
            _ => ShellResponse::CommandOutput { stdout: format!("Command '{}' not found.
", command), stderr: String::new(), exit_code: 127 },
        }
    }


    fn handle_request(&mut self, request: ShellRequest) -> ShellResponse {
        match request {
            ShellRequest::ExecuteCommand { command, args } => {
                self.command_history.push(format!("{} {}", command, args.join(" ")));
                log(&alloc::format!("Shell: Executing command: {} with args: {:?}", command, args));

                let full_cmd_line = format!("{} {}", command, args.join(" "));
                if full_cmd_line.contains("|") {
                    // Basic piping logic: split into two commands and simulate pipe
                    let parts: Vec<&str> = full_cmd_line.split('|').collect();
                    if parts.len() == 2 {
                        let cmd1_line = parts[0].trim();
                        let cmd2_line = parts[1].trim();

                        let (cmd1, args1) = Self::parse_single_command_line(cmd1_line);
                        let first_response = self.execute_internal_command(cmd1, args1);

                        if let ShellResponse::CommandOutput { stdout, .. } = first_response {
                            // Pass stdout of first command as a new argument to the second command
                            let (cmd2, mut args2) = Self::parse_single_command_line(cmd2_line);
                            if !stdout.is_empty() {
                                args2.insert(0, stdout.trim().to_string()); // Prepend stdout as first argument
                            }
                            return self.execute_internal_command(cmd2, args2);
                        } else if let ShellResponse::Error { message } = first_response {
                            return ShellResponse::Error(format!("Pipe error (cmd1): {}", message));
                        } else {
                            return ShellResponse::Error("Pipe error: unexpected response from first command.".to_string());
                        }
                    } else {
                        return ShellResponse::Error("shell: Only simple piping (cmd1 | cmd2) supported for now.".to_string());
                    }
                } else {
                    // No piping, execute as a single command
                    return self.execute_internal_command(command, args);
                }
            },
            ShellRequest::ChangeDirectory { path } => {
                self.handle_change_directory(path)
            },
            ShellRequest::GetCurrentDirectory => {
                ShellResponse::CurrentDirectory(self.current_dir.clone())
            },
            ShellRequest::RunLoggerCommand { message, level } => {
                match self.logger_chan.send_and_recv::<LoggerRequest, LoggerResponse>(&LoggerRequest::Log { message: message.clone(), level }) {
                    Ok(LoggerResponse::Success) => ShellResponse::LoggerResult { success: true },
                    Ok(LoggerResponse::Error(err_msg)) => ShellResponse::Error(format!("logger error: {}", err_msg)),
                    _ => ShellResponse::Error("logger: Unexpected response from Logger V-Node".to_string()),
                }
            },
            ShellRequest::RunEchoCommand { message } => {
                match self.echo_chan.send_and_recv::<EchoRequest, EchoResponse>(&EchoRequest::Echo { message: message.clone() }) {
                    Ok(EchoResponse::EchoReply { message: reply }) => ShellResponse::EchoResult { reply },
                    Ok(EchoResponse::Error(err_msg)) => ShellResponse::Error(format!("echo error: {}", err_msg)),
                    _ => ShellResponse::Error("echo: Unexpected response from Echo V-Node".to_string()),
                }
            },
            ShellRequest::RunTestCommand { test_name, args } => {
                let request = match test_name.as_str() {
                    "echo" => TestRequest::RunEchoTest { message: args.get(0).cloned().unwrap_or_default() },
                    "logger" => TestRequest::RunLoggerTest { 
                        message: args.get(0).cloned().unwrap_or_default(), 
                        level: LogLevel::Info, // Default level for now, could be parsed from args
                    },
                    _ => return ShellResponse::Error(format!("test: Unknown test '{}'.", test_name)),
                };

                match self.test_chan.send_and_recv::<TestRequest, TestResponse>(&request) {
                    Ok(TestResponse::EchoResult { reply }) => ShellResponse::TestResult { stdout: format!("Echo Test Reply: {}", reply), stderr: String::new(), success: true },
                    Ok(TestResponse::LoggerResult { success }) => ShellResponse::TestResult { stdout: format!("Logger Test Success: {}", success), stderr: String::new(), success },
                    Ok(TestResponse::Error(err_msg)) => ShellResponse::TestResult { stdout: String::new(), stderr: format!("test error: {}", err_msg), success: false },
                    _ => ShellResponse::Error("test: Unexpected response from Test V-Node".to_string()),
                }
            },
        }
    }

    fn handle_change_directory(&mut self, path: String) -> ShellResponse {
        // Conceptual: Validate path with VFS or simplify
        // For now, allow any path for simplicity
        // In a real system, would check if path is a directory and exists
        if path == ".." {
            // Go up one level
            if let Some(last_slash) = self.current_dir.rfind('/') {
                if last_slash == 0 && self.current_dir.len() > 1 {
                    self.current_dir = String::from("/");
                } else if last_slash > 0 {
                    self.current_dir.truncate(last_slash);
                }
            }
        } else if path.starts_with('/') {
            self.current_dir = path;
        } else {
            // Relative path
            if !self.current_dir.ends_with('/') {
                self.current_dir.push('/');
            }
            self.current_dir.push_str(&path);
        }
        ShellResponse::Success(format!("Changed directory to {}", self.current_dir))
    }

    fn run_loop(&mut self) -> ! {
        log("Shell Service: Entering main event loop.");
        loop {
            // Process incoming requests from client V-Nodes
            if let Ok(Some(req_data)) = self.client_chan.recv_non_blocking() {
                if let Ok(request) = postcard::from_bytes::<ShellRequest>(&req_data) {
                    log(&alloc::format!("Shell Service: Received ShellRequest: {:?}", request));
                    let response = self.handle_request(request);
                    self.client_chan.send(&response).unwrap_or_else(|_| log("Shell Service: Failed to send response to client."));
                } else {
                    log("Shell Service: Failed to deserialize ShellRequest.");
                }
            }

            // Yield to other V-Nodes to prevent busy-waiting
            unsafe { syscall3(SYS_TIME, 0, 0, 0); }
        }
    }
}

#[no_mangle]
pub extern "C" fn _start() -> ! {
    init_allocator();
    // Assuming channel IDs:
    // 8 for Shell Service client requests (e.g., AetherTerminal)
    // 7 for VFS Service
    // 6 for Init Service
    // 5 for DNS Resolver
    // 9 for Logger Service
    // 10 for Echo Service
    // 11 for Test Service
    let mut shell_service = ShellService::new(8, 7, 6, 5, 9, 10, 11);
    shell_service.run_loop();
}

#[panic_handler]
fn panic(_info: &PanicInfo) -> ! {
    log("Shell V-Node panicked!");
    loop {}
}
