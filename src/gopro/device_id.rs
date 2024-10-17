//! GoPro device ID (`DVID`).

use crate::FourCC;

/// GoPro device ID.
/// For older devices (Hero5, Fusion?) it seems
/// DVID can be either a `u32` or a `FourCC` (?).
#[derive(Debug, Clone)]
pub enum Dvid {
    Uint32(u32),
    FourCC(FourCC),
}

impl Default for Dvid {
    fn default() -> Self {
        Self::FourCC(FourCC::Invalid)
    }
}

impl std::fmt::Display for Dvid {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Dvid::Uint32(num) => write!(f, "{num}"),
            Dvid::FourCC(fourcc) => write!(f, "{}", fourcc.to_str()),
        }
    }
}

impl Into<Option<u32>> for &Dvid {
    fn into(self) -> Option<u32> {
        match self {
            Dvid::Uint32(n) => Some(*n),
            Dvid::FourCC(_) => None,
        }
    }
}

impl Into<Option<FourCC>> for &Dvid {
    fn into(self) -> Option<FourCC> {
        match self {
            Dvid::Uint32(_) => None,
            Dvid::FourCC(f) => Some(f.to_owned()),
        }
    }
}

impl Into<Option<String>> for &Dvid {
    fn into(self) -> Option<String> {
        match self {
            Dvid::Uint32(_) => None,
            Dvid::FourCC(f) => Some(f.to_str().to_owned()),
        }
    }
}
