//! GoPro GPMF data format core structs and methods.

pub mod gpmf;
pub mod fourcc;
pub mod header;
pub mod stream;
pub mod timestamp;
pub mod value;

pub use gpmf::Gpmf;
pub use fourcc::FourCC;
pub use stream::{Stream, StreamType};
pub use timestamp::Timestamp;
pub use value::Value;
pub use header::Header;