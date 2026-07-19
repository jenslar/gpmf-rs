//! Processing of GPS and various kinds of sensor data.

use time::{OffsetDateTime, PrimitiveDateTime, format_description};

use crate::GpmfError;

pub mod data_type;
pub mod gps;
pub mod imu;

pub use data_type::DataType;
pub use gps::{GoProPoint, Gps};
pub use imu::{ImuOrientation, Imu, ImuQuantifier, ImuSample, ImuType};

/// String representation for datetime objects.
pub(crate) fn primitivedatetime_to_string(datetime: &PrimitiveDateTime) -> Result<String, GpmfError> {
    // PrimitiveDateTime::to_string(&self.datetime) // sufficient?
    let format = format_description::parse("[year]-[month]-[day]T[hour]:[minute]:[second]")
        .map_err(|e| GpmfError::TimeError(e.into()))?;
    datetime.format(&format)
        .map_err(|e| GpmfError::TimeError(e.into()))
}

/// String representation for datetime objects.
/// Note that utc offset won't be used, since GoPro does not log this.
pub(crate) fn offsetdatetime_to_string(datetime: &OffsetDateTime) -> Result<String, GpmfError> {
    // PrimitiveDateTime::to_string(&self.datetime) // sufficient?
    let format = format_description::parse("[year]-[month]-[day]T[hour]:[minute]:[second]")
        .map_err(|e| GpmfError::TimeError(e.into()))?;
    datetime.format(&format)
        .map_err(|e| GpmfError::TimeError(e.into()))
}
