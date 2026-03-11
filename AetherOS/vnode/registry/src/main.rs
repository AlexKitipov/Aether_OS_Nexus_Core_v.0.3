use common::ipc::vnode::VNodeChannel;
use common::syscall::{syscall3, SYS_LOG};

fn log(msg: &str) {
    let _ = syscall3(SYS_LOG, msg.as_ptr() as u64, msg.len() as u64, 0);
}

fn main() {
    let mut own_chan = VNodeChannel::new(1);
    log("Registry V-Node starting up...");

    loop {
        if own_chan.recv_blocking().is_err() {
            log("Registry: receive error");
        }
    }
}
