use super::AdaptedDriver;
use crate::sandbox::DriverModel;

pub fn wrap_abi(_drv: AdaptedDriver) -> DriverModel {
    DriverModel::default()
}
