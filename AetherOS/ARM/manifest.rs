pub struct AppManifest {
    pub name: String,
    pub version: String,
}

pub fn load_manifest() -> AppManifest {
    AppManifest {
        name: "app".into(),
        version: "0.1".into(),
    }
}
