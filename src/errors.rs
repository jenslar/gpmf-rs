//! Various GPMF-related errors.

use std::{fmt, path::PathBuf};

use crate::gopro::GoProFileType;

/// Various GPMF related read/parse errors.
#[derive(Debug)]
pub enum GpmfError {
    /// Error parsing MP4.
    Mp4Error(mp4iter::errors::Mp4Error),
    /// Error parsing JPEG.
    JpegError(jpegiter::JpegError),
    /// Failed to locate GoPro offsets in MP4.
    // NoMp4Offsets(mp4iter::FourCC),
    NoMp4Offsets(String),
    /// Converted `BinResult` error.
    BinReadError(binrw::Error),
    /// Converted `time::Error` error.
    TimeError(time::Error),
    /// Converted `Utf8Error`.
    Utf8Error(std::string::FromUtf8Error),
    /// Parse integer from string error.
    ParseIntError(std::num::ParseIntError),
    /// Generic GPMF parse error
    ParseError,
    /// IO error
    DowncastIntError(std::num::TryFromIntError),
    /// Failed to cast source type into target type.
    IOError(std::io::Error),
    /// Filesizes of e.g. 0 sized place holders.
    ReadMismatch{got: u64, expected: u64},
    /// Seek mismatch.
    OffsetMismatch{got: u64, expected: u64},
    /// MP4 0 sized atoms,
    /// e.g. 1k Dropbox place holders.
    UnexpectedAtomSize{len: u64, offset: u64},
    /// No such atom.
    NoSuchAtom(String),
    /// MP4 ouf of bounds.
    BoundsError((u64, u64)),
    /// Filesizes of e.g. 0 sized place holders.
    MaxFileSizeExceeded{max: u64, got: u64, path: PathBuf},
    /// Unknown base type when parsing `Values`.
    UnknownBaseType(u8),
    /// Missing type definition for Complex type (`63`/`?`)
    MissingComplexType,
    /// Exceeded recurse depth when parsing GPMF into `Stream`s
    RecurseDepthExceeded((usize, usize)),
    /// Invalid FourCC. For detecting `&[0, 0, 0, 0]`.
    /// E.g. GoPro `udta` atom contains
    /// mainly undocumented GPMF data and is padded with
    /// zeros.
    InvalidFourCC,
    /// Failed to find MUID
    NoMuid,
    /// Failed to find GUMI
    NoGumi,
    /// For handling GPMF sources, when e.g. an MP4-file
    /// was expected but another file type was passed.
    InvalidFileType(PathBuf),
    InvalidGoProFileType(GoProFileType),
    /// Missing path (e.g. no path set for `GoProFile`)
    PathNotSet,
    /// Missing high-res video path (e.g. no MP4 set for `GoProFile`)
    HighResVideoNotSet,
    /// Missing low-res video path (e.g. no LRV set for `GoProFile`)
    LowResVideoNotSet,
    /// Error if attempting to merge two GoPro files
    /// with different fingerprints.
    FingerprintMismatch,
    /// Model or camera not known,
    /// mostly for generic MP4 files with no identifiers.
    UknownDevice,
    /// No data for requested type (e.g. no GPS logged)
    NoData,
    /// No recording session
    NoSession,
    /// Failed to determine path of parent dir.
    NoParentDir
}

impl std::error::Error for GpmfError {} // not required?

impl fmt::Display for GpmfError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            GpmfError::Mp4Error(err) => write!(f, "{err}"),
            GpmfError::JpegError(err) => write!(f, "{err}"),
            // GpmfError::NoMp4Offsets => write!(f, "Failed to locate GoPro GPMF offsets in MP4."),
            GpmfError::NoMp4Offsets(name) => write!(f, "Failed to locate GoPro GPMF offsets in MP4 for handler with name '{name}'."),
            GpmfError::BinReadError(err) => write!(f, "{err}"),
            GpmfError::TimeError(err) => write!(f, "{err}"),
            GpmfError::Utf8Error(err) => write!(f, "{err}"),
            GpmfError::ParseIntError(err) => write!(f, "Unable to parse string into integer: {}", err),
            GpmfError::ParseError => write!(f, "Failed to parse GPMF data."),
            GpmfError::DowncastIntError(err) => write!(f, "Failed to downcast integer: {err}"),
            GpmfError::IOError(err) => write!(f, "IO error: {}", err),
            GpmfError::ReadMismatch{got, expected} => write!(f, "Read {got} bytes, expected {expected} bytes."),
            GpmfError::OffsetMismatch{got, expected} => write!(f, "Moved {got} bytes, expected to move {expected} bytes"),
            GpmfError::UnexpectedAtomSize{len, offset} => write!(f, "Unexpected MP4 atom size of {len} bytes @ offset {offset}."),
            GpmfError::NoSuchAtom(name) => write!(f, "No such atom {name}."),
            GpmfError::BoundsError((got, max)) => write!(f, "Bounds error: tried to read file at {got} with max {max}."),
            GpmfError::MaxFileSizeExceeded {max, got, path} => write!(f, "{} ({got} bytes) exceeds maximum file size of {max}.", path.display()),
            GpmfError::UnknownBaseType(bt) => write!(f, "Unknown base type {}/'{}'", bt, *bt as char),
            GpmfError::MissingComplexType => write!(f, "Missing type definitions for complex type '?'"),
            GpmfError::RecurseDepthExceeded((depth, max)) => write!(f, "Recurse depth {depth} exceeds max recurse depth {max}"),
            GpmfError::InvalidFourCC => write!(f, "Invalid FourCC"),
            GpmfError::NoMuid => write!(f, "No MUID found"),
            GpmfError::NoGumi => write!(f, "No GUMI found"),
            GpmfError::InvalidGoProFileType(filetype) => write!(f, "Can not use {filetype:?} for this action"),
            GpmfError::InvalidFileType(path) => write!(f, "Invalid file type: '{}'", path.display()),
            GpmfError::PathNotSet => write!(f, "Path not set"),
            GpmfError::HighResVideoNotSet => write!(f, "Path for high-resolution clip not set"),
            GpmfError::LowResVideoNotSet => write!(f, "Path for low-resolution clip not set"),
            GpmfError::FingerprintMismatch => write!(f, "GoProFile merge failed: fingerprint mismatch."),
            GpmfError::UknownDevice => write!(f, "Unknown device"),
            GpmfError::NoData => write!(f, "No data for requested type"),
            GpmfError::NoSession => write!(f, "No session for specified MP4"),
            GpmfError::NoParentDir => write!(f, "Failed to determine path of parent dir."),
        }
    }
}

/// Converts std::io::Error to GpmfError
impl From<std::io::Error> for GpmfError {
    fn from(err: std::io::Error) -> Self {
        GpmfError::IOError(err)
    }
}

/// Converts std::string::FromUtf8Error to GpmfError
/// (`&str` reqiures `std::str::Utf8Error`)
impl From<std::string::FromUtf8Error> for GpmfError {
    fn from(err: std::string::FromUtf8Error) -> GpmfError {
        GpmfError::Utf8Error(err)
    }
}

/// Converts std::num::ParseIntError to GpmfError
impl From<std::num::ParseIntError> for GpmfError {
    fn from(err: std::num::ParseIntError) -> GpmfError {
        GpmfError::ParseIntError(err)
    }
}

/// Converts mp4iter::errors::Mp4Error to GpmfError
impl From<mp4iter::errors::Mp4Error> for GpmfError {
    fn from(err: mp4iter::errors::Mp4Error) -> GpmfError {
        GpmfError::Mp4Error(err)
    }
}
/// Converts binread::Error to FitError
impl From<binrw::Error> for GpmfError {
    fn from(err: binrw::Error) -> GpmfError {
        GpmfError::BinReadError(err)
    }
}

/// Converts time::Error to GpmfError
impl From<time::Error> for GpmfError {
    fn from(err: time::Error) -> GpmfError {
        GpmfError::TimeError(err)
    }
}

/// Converts GpmfError to std::io::Error
impl From<GpmfError> for std::io::Error {
    fn from(err: GpmfError) -> Self {
        std::io::Error::new(std::io::ErrorKind::Other, err)
    }
}
