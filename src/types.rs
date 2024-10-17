/// Media Unique ID.
/// Same across clips in the same session.
/// Can be used as session ID.
pub type Muid = [u32; 8];
/// Global Unique ID.
/// For newer devices this is set to `[0, 0, 0, 0]`
/// for the first low resolution clip only (`.LRV`).
/// Remaining low-resolution clips will have the same value
/// as the high-resolution clips.
/// The high-resolution video always has GUMI set.
/// For older devices GUMI is set for all clips.
pub type Gumi = [u32; 4];