use crate::protocol::GatewayMessage;

pub trait KernelBridge {
    type Error;

    fn recv_kernel_message(&mut self) -> Result<GatewayMessage, Self::Error>;
    fn send_kernel_message(&mut self, message: GatewayMessage) -> Result<(), Self::Error>;
}
