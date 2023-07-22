//! GoPro device name (`DVNM`).

use std::path::Path;

use crate::GpmfError;

/// GoPro camera model. Set in GPMF struct for convenience.
/// Does not yet include all previous models, hence `Other<String>`
// #[derive(Debug, Clone, Eq, Hash)]
// #[derive(Debug, Clone, PartialEq, Ord)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DeviceName {
    Hero5Black,  // DVNM not confirmed
    Hero6Black,  // DVNM not confirmed
    Hero7Black,  // DVNM "Hero7 Black" or "HERO7 Black" (MP4 GoPro MET udta>minf atom)
    Hero8Black,  // probably "Hero7 Black", but not confirmed
    Hero9Black,  // DVNM "Hero9 Black" or "HERO9 Black" (MP4 GoPro MET udta>minf atom)
    Hero10Black, // DVNM "Hero10 Black" or "HERO10 Black" (MP4 GoPro MET udta>minf atom)
    Hero11Black, // DVNM "Hero11 Black" or "HERO11 Black" (MP4 GoPro MET udta>minf atom)
    Fusion,
    GoProMax,
    GoProKarma,  // DVNM "GoPro Karma v1.0" + whichever device is connected e.g. hero 5.
    // other identifiers? Silver ranges etc?
    // Other(String), // for models not yet included as enum
}

impl Default for DeviceName {
    fn default() -> Self {
        // Self::Other("Unknown".to_owned())
        // Use first GPMF hero as default
        Self::Hero5Black
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
        mp4.reset()?;
        let udta = mp4.udta(true)?;
        if let Some(field) = udta.find("FIRM") {
            if let Some(id) = field.to_string() {
                match Self::from_firmware_id(&id) {
                    Some(dvnm) => return Ok(dvnm),
                    None => return Err(GpmfError::UknownDevice)
                }
            }
        }

        Err(GpmfError::UknownDevice)
    }

    pub fn from_firmware_id(id: &str) -> Option<Self> {
        match &id[..3] {
            "HD5" => Some(Self::Hero5Black),
            "HD6" => Some(Self::Hero6Black),
            "FS1" => Some(Self::Fusion),
            "HD7" => Some(Self::Hero7Black),
            "HD8" => Some(Self::Hero8Black),
            "HD9" => Some(Self::Hero9Black), // possibly H20
            "H19" => Some(Self::GoProMax),
            "H20" => Some(Self::Hero9Black), // possibly HD9, and H20 is another device
            "H21" => Some(Self::Hero10Black),
            "H22" => Some(Self::Hero11Black),
            _ => None
        }
    }

    pub fn from_str(model: &str) -> Option<Self> {
        match model.trim() {
            // Hero5 Black identifies itself as "Camera" so far.
            "Camera" | "Hero5 Black" | "HERO5 Black" => Some(Self::Hero5Black),
            "Hero6 Black" | "HERO6 Black" => Some(Self::Hero6Black),
            "Hero7 Black" | "HERO7 Black" => Some(Self::Hero7Black),
            "Hero8 Black" | "HERO8 Black" => Some(Self::Hero8Black),
            "Hero9 Black" | "HERO9 Black" => Some(Self::Hero9Black),
            "Hero10 Black" | "HERO10 Black" => Some(Self::Hero10Black),
            "Hero11 Black" | "HERO11 Black" => Some(Self::Hero11Black),
            "Fusion" | "FUSION" => Some(Self::Fusion),
            "GoPro Max" => Some(Self::GoProMax),
            "GoPro Karma v1.0" => Some(Self::GoProKarma),
            _ => None
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
            Self::Fusion => "Fusion",
            Self::GoProMax => "GoPro Max",
            Self::GoProKarma => "GoPro Karma v1.0", // only v1.0 so far
        }
    }

    // Get documented sample frequency for a specific device
    // pub fn freq(&self, fourcc: FourCC) {
    //     match self {

    //     }
    // }
}
