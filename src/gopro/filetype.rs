#[derive(Debug, PartialEq)]
/// High (`MP4`), low (`LRV`),
/// or either resolution (`ANY`).
pub enum GoProFileType {
    /// High-resolution GoPro clip (`.MP4`)
    High,
    /// Low-resolution GoPro clip (`.LRV`)
    Low,
    /// Either LRV or MP4 GoPro clip
    Any
}