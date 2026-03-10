pub mod ui_protocol;
pub mod vfs_ipc;
pub mod vnode;
pub mod logger_ipc;
pub mod echo_ipc;
pub mod test_ipc;
pub mod file_manager_ipc;
pub mod shell_ipc;

pub trait IpcSend {
    fn send_raw(&mut self, bytes: &[u8]) -> Result<(), ()>;

    fn send<T: serde::Serialize>(&mut self, msg: &T) -> Result<(), ()> {
        let serialized = postcard::to_allocvec(msg).map_err(|_| ())?;
        self.send_raw(&serialized)
    }
}

pub trait IpcRecv {
    fn recv<T: serde::de::DeserializeOwned>(&mut self) -> Option<T>;
}
