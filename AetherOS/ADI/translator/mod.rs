use crate::analyzer::AnalyzedDriver;
use crate::sandbox::DriverModel;

pub struct Translator;

impl Translator {
    pub fn translate(_drv: AnalyzedDriver) -> DriverModel {
        DriverModel::default()
    }
}
