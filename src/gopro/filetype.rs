#[derive(Debug, PartialEq)]
/// High (`MP4`), low (`LRV`),
/// or either resolution (`ANY`).
pub enum GoProFileType {
    /// High-resolution GoPro clip
    MP4,
    /// Low-resolution GoPro clip
    LRV,
    /// Either LRV or MP4 GoPro clip
    ANY
}