//! GoPro device name (`DVNM`).

use std::{fmt::Display, path::Path};

use crate::GpmfError;

/// GoPro camera model. Set in GPMF struct for convenience.
/// Does not yet include all previous models, hence `Other<String>`
// #[derive(Debug, Clone, Eq, Hash)]
// #[derive(Debug, Clone, PartialEq, Ord)]
#[derive(Debug, Default, Clone, PartialEq, Eq, Hash)]
pub enum DeviceName {
    #[default]
    Hero5Black,  // DVNM not confirmed
    Hero6Black,  // DVNM not confirmed
    Hero7Black,  // DVNM "Hero7 Black" or "HERO7 Black" (MP4 GoPro MET udta>minf atom)
    Hero8Black,  // probably "Hero7 Black", but not confirmed
    Hero9Black,  // DVNM "Hero9 Black" or "HERO9 Black" (MP4 GoPro MET udta>minf atom)
    Hero10Black, // DVNM "Hero10 Black" or "HERO10 Black" (MP4 GoPro MET udta>minf atom)
    Hero11Black, // DVNM "Hero11 Black" or "HERO11 Black" (MP4 GoPro MET udta>minf atom)
    Hero12Black, // DVNM "Hero12 Black" or "HERO12 Black" (MP4 GoPro MET udta>minf atom)
    // Hero13Black, // DVNM "Hero12 Black" or "HERO12 Black" (MP4 GoPro MET udta>minf atom)
    Fusion,
    GoProMax,
    GoProKarma,  // DVNM "GoPro Karma v1.0" + whichever device is connected e.g. hero 5.
    Unknown,
}

impl Display for DeviceName {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.to_str())
    }
}

impl DeviceName {
    /// Try to determine model from start of `mdat`, which contains
    /// data/fields similar to those in the `udta` atom.
    ///
    /// `GPRO` should immediately follow the `mdat` header,
    /// then 4 bytes representing size of the section (`u32` Little Endian).
    /// Currently using the start of the firmware string as id (e.g. HD8 = Hero8 Black),
    /// but the full device name string exists as a string a bit later after other fields.
    pub fn from_path(path: &Path) -> Result<Self, GpmfError> {
        let mut mp4 = mp4iter::Mp4::new(path)?;
        Self::from_file(&mut mp4)
    }

    pub(crate) fn from_file(mp4: &mut mp4iter::Mp4) -> Result<Self, GpmfError> {
        let mut firm = mp4.find_user_data("FIRM")?;
        firm.read_to_string().map(|s| Self::from_firmware_id(&s))
            .map_err(|e| e.into())
    }

    pub fn from_firmware_id(id: &str) -> Self {
        match &id[..3] {
            "HD5" => Self::Hero5Black,
            "HD6" => Self::Hero6Black,
            "FS1" => Self::Fusion,
            "HD7" => Self::Hero7Black,
            "HD8" => Self::Hero8Black,
            "HD9" => Self::Hero9Black, // possibly H20
            "H19" => Self::GoProMax,
            "H20" => Self::Hero9Black, // possibly HD9, and H20 is another device
            "H21" => Self::Hero10Black,
            "H22" => Self::Hero11Black,
            "H23" => Self::Hero12Black,
            _ => Self::Unknown
        }
    }

    pub fn from_str(model: &str) -> Self {
        match model.trim() {
            // Hero5 Black identifies itself as "Camera" so far.
            "Camera" | "Hero5 Black" | "HERO5 Black" => Self::Hero5Black,
            "Hero6 Black" | "HERO6 Black" => Self::Hero6Black,
            "Hero7 Black" | "HERO7 Black" => Self::Hero7Black,
            "Hero8 Black" | "HERO8 Black" => Self::Hero8Black,
            "Hero9 Black" | "HERO9 Black" => Self::Hero9Black,
            "Hero10 Black" | "HERO10 Black" => Self::Hero10Black,
            "Hero11 Black" | "HERO11 Black" => Self::Hero11Black,
            "Hero12 Black" | "HERO12 Black" => Self::Hero12Black,
            "Fusion" | "FUSION" => Self::Fusion,
            "GoPro Max" => Self::GoProMax,
            "GoPro Karma v1.0" => Self::GoProKarma,
            _ => Self::Unknown
        }
    }

    pub fn to_str(&self) -> &str {
        match self {
            Self::Hero5Black => "Hero5 Black", // correct device name?
            Self::Hero6Black => "Hero6 Black", // correct device name?
            Self::Hero7Black => "Hero7 Black",
            Self::Hero8Black => "Hero8 Black",
            Self::Hero9Black => "Hero9 Black",
            Self::Hero10Black => "Hero10 Black",
            Self::Hero11Black => "Hero11 Black",
            Self::Hero12Black => "Hero12 Black",
            Self::Fusion => "Fusion",
            Self::GoProMax => "GoPro Max",
            Self::GoProKarma => "GoPro Karma v1.0", // only v1.0 so far
            Self::Unknown => "Unknown", // only v1.0 so far
        }
    }

    // Get documented sample frequency for a specific device
    // pub fn freq(&self, fourcc: FourCC) {
    //     match self {

    //     }
    // }
}
