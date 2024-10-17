use time::{macros::datetime, PrimitiveDateTime};

pub const GOPRO_DATETIME_DEFAULT: PrimitiveDateTime = datetime!(2000-1-1 0:0:0);
pub const GOPRO_METADATA_HANDLER: &'static str = "GoPro MET";
pub const GOPRO_AUDIO_HANDLER: &'static str = "GoPro AAC";
pub const GOPRO_H265_HANDLER: &'static str = "GoPro H.265";
/// hdlr atom handler name
pub const GOPRO_TIMECODE_HANDLER: &'static str = "GoPro TCD";
pub const GOPRO_UDTA_GPMF_FOURCC: &'static str = "GPMF";
/// Min resolution. Lower than this means it is a LRV-file.
pub const GOPRO_MIN_WIDTH_HEIGHT: (u16, u16) = (1920, 1080);