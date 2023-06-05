//! Various GoPro related structs and methods.

pub mod device_name;
pub mod device_id;
pub mod file;
pub mod filetype;
pub mod session;
pub mod meta;

pub use filetype::GoProFileType;
pub use file::GoProFile;
pub use session::GoProSession;
pub use meta::GoProMeta;
pub use device_id::Dvid;
pub use device_name::DeviceName;
