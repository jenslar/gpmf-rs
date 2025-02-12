use time::{macros::datetime, PrimitiveDateTime};

/// Track name (`hdlr` atom handler name) for GoPro start time (earliest time GoPro supports)
pub const GOPRO_DATETIME_DEFAULT: PrimitiveDateTime = datetime!(2000-1-1 0:0:0);
/// Track name (`hdlr` atom handler name) for GoPro timed telemetry GPMF track
pub const GOPRO_METADATA_HANDLER: &'static str = "GoPro MET";
/// Track name (`hdlr` atom handler name) for GoPro audio track
pub const GOPRO_AUDIO_HANDLER: &'static str = "GoPro AAC";
/// Track name (`hdlr` atom handler name) for GoPro video track
/// on devices that record in H265.
pub const GOPRO_H265_HANDLER: &'static str = "GoPro H.265";
/// Track name (`hdlr` atom handler name) for GoPro time code track
pub const GOPRO_TIMECODE_HANDLER: &'static str = "GoPro TCD";
/// Atom FourCC in `udta`` atom for GoPro metadata in GPMF format
pub const GOPRO_UDTA_GPMF_FOURCC: &'static str = "GPMF";
/// Min resolution threshold for high resolution GoPro video.
/// Lower than this means it is a low-resolution vide
/// (i.e. LRV-file meant for on-device viewing).
pub const GOPRO_MIN_WIDTH_HEIGHT: (u16, u16) = (1920, 1080);