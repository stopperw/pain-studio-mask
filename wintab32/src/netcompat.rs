use crate::ffi::Axis;

impl From<psm_common::netcode::Axis> for Axis {
    fn from(value: psm_common::netcode::Axis) -> Self {
        Self {
            min: value.min,
            max: value.max,
            units: value.units,
            resolution: value.resolution,
        }
    }
}
