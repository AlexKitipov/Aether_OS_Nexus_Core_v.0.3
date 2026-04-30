#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct DeviceScanResult {
    pub device_found: bool,
}

pub fn scan_device() -> DeviceScanResult {
    DeviceScanResult { device_found: false }
}
