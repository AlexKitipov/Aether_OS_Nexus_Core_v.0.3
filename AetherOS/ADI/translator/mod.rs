pub mod mapper;
pub mod adapter;
pub mod abi;

pub use mapper::*;
pub use adapter::*;
pub use abi::*;

use crate::analyzer::AnalyzedDriver;
use crate::sandbox::DriverModel;

pub struct Translator;

impl Translator {
    pub fn translate(drv: AnalyzedDriver) -> DriverModel {
        let mapped = map_calls(&drv);
        let adapted = adapt(mapped);
        wrap_abi(adapted)
    }
}
