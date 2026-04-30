pub struct IpcMessage;

pub fn ipc_send(_msg: IpcMessage) {}

pub fn ipc_recv() -> IpcMessage {
    IpcMessage
}
