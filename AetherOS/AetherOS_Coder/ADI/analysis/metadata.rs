#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct DriverMetadata {
    pub name: &'static str,
    pub version: u32,
}

pub fn extract_metadata() -> DriverMetadata {
    DriverMetadata {
        name: "unknown",
        version: 0,
    }
}
