use crate::protocol::GatewayMessage;

pub trait KernelBridge {
    type Error;

    fn recv_kernel_message(&mut self) -> Result<GatewayMessage, Self::Error>;
    fn send_kernel_message(&mut self, message: GatewayMessage) -> Result<(), Self::Error>;
}

/// Adapter over kernel IPC send/recv functions.
pub struct IpcKernelBridge<Recv, Send, E>
where
    Recv: FnMut() -> Result<GatewayMessage, E>,
    Send: FnMut(GatewayMessage) -> Result<(), E>,
{
    recv_fn: Recv,
    send_fn: Send,
}

impl<Recv, Send, E> IpcKernelBridge<Recv, Send, E>
where
    Recv: FnMut() -> Result<GatewayMessage, E>,
    Send: FnMut(GatewayMessage) -> Result<(), E>,
{
    pub fn new(recv_fn: Recv, send_fn: Send) -> Self {
        Self { recv_fn, send_fn }
    }
}

impl<Recv, Send, E> KernelBridge for IpcKernelBridge<Recv, Send, E>
where
    Recv: FnMut() -> Result<GatewayMessage, E>,
    Send: FnMut(GatewayMessage) -> Result<(), E>,
{
    type Error = E;

    fn recv_kernel_message(&mut self) -> Result<GatewayMessage, Self::Error> {
        (self.recv_fn)()
    }

    fn send_kernel_message(&mut self, message: GatewayMessage) -> Result<(), Self::Error> {
        (self.send_fn)(message)
    }
}
