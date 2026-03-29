pub mod types;
pub mod ui_protocol;
pub mod display;
pub mod webview;
pub mod vfs_ipc;
pub mod vnode;
pub mod logger_ipc;
pub mod echo_ipc;
pub mod test_ipc;
pub mod file_manager_ipc;
pub mod shell_ipc;
pub mod init_ipc;
pub mod keyboard_ipc;
pub mod socket_ipc;
pub mod dns_ipc;

pub trait IpcSend {
    fn send_raw(&mut self, bytes: &[u8]) -> Result<(), ()>;

    #[cfg(feature = "serde")]
    fn send<T: serde::Serialize>(&mut self, msg: &T) -> Result<(), ()> {
        let serialized = postcard::to_allocvec(msg).map_err(|_| ())?;
        self.send_raw(&serialized)
    }
}

pub trait IpcRecv {
    fn recv_raw(&mut self) -> Option<alloc::vec::Vec<u8>>;

    #[cfg(feature = "serde")]
    fn recv<T: serde::de::DeserializeOwned>(&mut self) -> Option<T> {
        self.recv_raw()
            .and_then(|data| postcard::from_bytes(&data).ok())
    }
}
pub mod model_runtime_ipc;
pub mod mail_ipc;
pub mod ai_governor_ipc;


pub use types::IpcMessage;
