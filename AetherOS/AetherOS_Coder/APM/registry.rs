use alloc::string::String;
use alloc::vec;
use alloc::vec::Vec;

pub struct AppRegistry;

impl AppRegistry {
    pub fn register(_manifest: super::AppManifest) {}
    pub fn list() -> Vec<String> { vec![] }
}
