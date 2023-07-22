//! Parse GoPro GPMF data. Returned in unprocessed form for most data types.
//! Processing of GPS data is supported,
//! whereas processing of sensor data into more common forms will be added gradually.
//! 
//! ```rs
//! use gpmf_rs::{Gpmf, SensorType};
//! use std::path::Path;
//! 
//! fn main() -> std::io::Result<()> {
//!     let path = Path::new("GOPRO_VIDEO.MP4");
//! 
//!     // Extract GPMF data
//!     let gpmf = Gpmf::new(&path)?;
//!     println!("{gpmf:#?}");
//! 
//!     // Filter and process GPS log, prune points that do not have at least a 2D fix
//!     let gps = gpmf.gps().prune(2);
//!     println!("{gps:#?}");
//! 
//!     // Filter and process accelerometer data.
//!     let sensor = gpmf.sensor(&SensorType::Accelerometer);
//!     println!("{sensor:#?}");
//! 
//!     Ok(())
//! }
//! ```

pub mod gpmf;
pub (crate) mod files;
mod errors;
mod content_types;
mod gopro;

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
