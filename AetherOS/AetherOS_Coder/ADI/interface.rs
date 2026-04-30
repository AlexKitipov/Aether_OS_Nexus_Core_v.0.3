use crate::analyzer::Analyzer;
use crate::sandbox::{Governor, Runtime};
use crate::translator::Translator;

pub struct ADIInterface;

impl ADIInterface {
    pub fn load_driver(device: DeviceInfo) -> Result<crate::sandbox::DriverModel, ADIError> {
        let analyzed = Analyzer::analyze(device.driver_source());
        let model = Translator::translate(analyzed);

        let _runtime = Runtime::new(64 * 1024 * 1024, 10_000_000, Governor::default());

        Ok(model)
    }

    pub fn start_driver(_vnode_id: u64) -> Result<(), ADIError> {
        Ok(())
    }

    pub fn stop_driver(_vnode_id: u64) -> Result<(), ADIError> {
        Ok(())
    }
}

#[derive(Clone, Copy)]
pub struct DeviceInfo {
    driver_src: &'static str,
}

impl DeviceInfo {
    pub const fn new(driver_src: &'static str) -> Self {
        Self { driver_src }
    }

    pub fn driver_source(self) -> &'static str {
        self.driver_src
    }
}

pub struct ADIError;
