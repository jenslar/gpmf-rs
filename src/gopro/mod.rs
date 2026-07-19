//! Various GoPro related structs and methods.

pub mod device_name;
pub mod device_id;
pub mod device_info;
// pub mod file_old;
pub mod file;
pub mod filetype;
// pub mod session_old;
pub mod session;
pub mod meta;

pub use filetype::GoProFileType;
// pub use file::GoProFileOld;
pub use file::GoProFile;
// pub use session::GoProSessionOld;
pub use session::{GoProSession, GoProMultiSession};
pub use meta::GoProMeta;
pub use device_id::DeviceId;
pub use device_name::DeviceName;
pub use device_info::DeviceInfo;
