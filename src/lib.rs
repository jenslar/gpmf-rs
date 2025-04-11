//! Parse GoPro GPMF data. Returned in unprocessed form for most data types.
//!
//! All GPMF data is supported as a "first pass parse", i.e. parsing the raw GPMF data according to
//! the specification at [https://github.com/gopro/gpmf-parser/blob/main/README.md](https://github.com/gopro/gpmf-parser/blob/main/README.md).
//!
//! Only GPS and sensor data is supported for processing into more useful forms.
//!
//! GPS support:
//! - GPS5 (Lat., Long., Alt., 2D speed, 3D speed):
//!     - all previous GoPro cameras with a GPS module
//!     - deprecated with Hero 11 Black
//! - GPS9 (Lat., Long., Alt., 2D, 3D, days, secs, DOP, fix):
//!     - Hero 11 (logs both GPS5 and GPS9)
//!     - Hero 13
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
//!     // Filter and process GPS log, prune points that do not have at least a 2D fix,
//!     // and a dilution of precision value of max 5.0.
//!     let gps = gpmf.gps().prune(2, Some(5.0));
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
mod constants;
mod types;

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
pub use constants::{*};
pub use types::{Muid, Gumi};
