use std::io::{BufRead, BufReader, Write};
use std::os::unix::net::UnixStream;

use crate::kernel_bridge::KernelBridge;
use crate::protocol::GatewayMessage;

pub fn run_gateway_loop<B>(bridge: &mut B, ai_socket_path: &str) -> Result<(), Box<dyn std::error::Error>>
where
    B: KernelBridge,
    B::Error: std::error::Error + 'static,
{
    let mut stream = UnixStream::connect(ai_socket_path)?;
    let mut reader = BufReader::new(stream.try_clone()?);

    loop {
        let kernel_message = bridge.recv_kernel_message()?;
        let payload = serde_json::to_vec(&kernel_message)?;
        stream.write_all(&payload)?;
        stream.write_all(b"\n")?;

        let mut line = String::new();
        let read = reader.read_line(&mut line)?;
        if read == 0 {
            continue;
        }

        let response: GatewayMessage = serde_json::from_str(line.trim())?;
        bridge.send_kernel_message(response)?;
    }
}
