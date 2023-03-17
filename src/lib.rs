//! Parse GoPro GPMF data. Returned in unprocessed form for most data types.
//! Processing of GPS data is supported,
//! whereas processing of sensor data into more common forms will be added gradually.
//! 
//! ```rs
//! use gpmf_rs::Gpmf;
//! use std::path::Path;
//! 
//! fn main() -> std::io::Result<()> {
//!     let path = Path::new("GOPRO_VIDEO.MP4");
//!     let gpmf = Gpmf::new(&path)?;
//!     Ok(())
//! }
//! ```

pub mod gpmf;
pub (crate) mod files;
mod errors;
mod content_types;
mod gopro;
mod geo;

pub use gpmf::{
    Gpmf,
    FourCC,
    Stream,
    StreamType,
    Timestamp
};
pub use content_types::{DataType,Gps, GoProPoint};
pub use content_types::sensor::{SensorData, SensorType};
pub use errors::GpmfError;
pub use gopro::GoProFile;
pub use gopro::GoProSession;
pub use gopro::DeviceName;
