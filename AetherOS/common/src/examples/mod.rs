#[derive(Clone)]
pub struct Manifest {
    pub root_cid: [u8; 32],
}

pub mod hello_package {
    use super::Manifest;

    pub fn make_hello_package() -> (Manifest, ()) {
        (Manifest { root_cid: [0; 32] }, ())
    }
}
