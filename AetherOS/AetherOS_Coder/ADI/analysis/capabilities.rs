use super::metadata::DriverMetadata;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct CapabilityInfo {
    pub supports_hotplug: bool,
}

pub fn detect_capabilities(meta: &DriverMetadata) -> CapabilityInfo {
    CapabilityInfo {
        supports_hotplug: meta.version > 0,
    }
}
