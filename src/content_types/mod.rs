//! Processing of GPS and various kinds of sensor data.

use time::{PrimitiveDateTime, format_description};

use crate::GpmfError;

pub mod data_types;
pub mod gps;
pub mod sensor;

pub use data_types::DataType;
pub use gps::{GoProPoint, Gps};

/// String representation for datetime objects.
pub(crate) fn primitivedatetime_to_string(datetime: &PrimitiveDateTime) -> Result<String, GpmfError> {
    // PrimitiveDateTime::to_string(&self.datetime) // sufficient?
    let format = format_description::parse("[year]-[month]-[day]T[hour]:[minute]:[second]")
        .map_err(|e| GpmfError::TimeError(e.into()))?;
    datetime.format(&format)
        .map_err(|e| GpmfError::TimeError(e.into()))
}