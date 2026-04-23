use super::manifest::{AppManifest, load_manifest};

pub fn install_app(_source: &str) -> AppManifest {
    load_manifest()
}
