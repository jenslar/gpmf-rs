//! GoPro device name (`DVNM`).
//!
//! Fusion: FUSION       FS1.04.01.70.00 ERROR on `--tree`: Failed to extract metadata from GoPro MP4: IO error: failed to fill whole buffer
//! Hero5:  HERO5 Black  HD5.02.02.00.00 (no gpmf section in udta)
//! Hero6:  HERO6 Black  HD6.01.01.60.00
//! Hero7:  HERO7 Black  HD7.01.01.80.00
//! Hero8:  HERO8 Black  HD8.01.01.20.00
//! Max1:   GoPro Max    H19.03.01.30.00 (same for 360 mode or hero mode)
//! Hero11: HERO11 Black H22.01.02.01.00
//! Hero12: HERO12 Black H23.01.02.20.00
//! Hero13: HERO13 Black H24.01.01.12.00 / FIRM ex: H24.01.01.20.00
//! Max2:   MAX2         H24.02.01.09.71

use std::{fmt::Display, path::Path};

use crate::{GpmfError, gopro::GoProMeta};

/// GoPro camera model.
/// Only Hero5 and later have GPMF data.
#[derive(Debug, Default, Clone, PartialEq, Eq, Hash)]
pub enum DeviceName {
    #[default]
    Hero2014,
    Hero2018,
    Hero2024,
    HeroSession,
    HeroPlus,
    HeroPlusLcd,
    Hero3Black,
    Hero3Silver,
    Hero3White,
    Hero3PlusSilver,
    Hero3PlusBlack,
    Hero4Black,
    Hero4Silver,
    Hero5Black,  // DVNM not confirmed
    Hero5Session,  // DVNM not confirmed
    Hero6Black,  // DVNM not confirmed
    Hero7Black,  // DVNM "Hero7 Black" or "HERO7 Black" (MP4 GoPro MET udta>minf atom)
    Hero7White,  // DVNM "Hero7 Black" or "HERO7 Black" (MP4 GoPro MET udta>minf atom)
    Hero7Silver,  // DVNM "Hero7 Black" or "HERO7 Black" (MP4 GoPro MET udta>minf atom)
    Hero8Black,  // probably "Hero8 Black" or "HERO8 Black", but not confirmed
    Hero9Black,  // DVNM "Hero9 Black" or "HERO9 Black" (MP4 GoPro MET udta>minf atom)
    Hero10Black, // DVNM "Hero10 Black" or "HERO10 Black" (MP4 GoPro MET udta>minf atom)
    Hero11Black, // DVNM "Hero11 Black" or "HERO11 Black" (MP4 GoPro MET udta>minf atom)
    Hero11BlackMini,
    Hero12Black, // DVNM "Hero12 Black" or "HERO12 Black" (MP4 GoPro MET udta>minf atom)
    Hero13Black, // DVNM "Hero13 Black" or "HERO13 Black" (MP4 GoPro MET udta>minf atom)
    HeroLit,
    Mission1,    // H25...?
    Mission1Pro, // H26.01
    Fusion,
    Max,
    Max2, // MAX2
    Karma,  // DVNM "GoPro Karma v1.0" + whichever device is connected e.g. hero 5.
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
        Self::from_mp4(&mut mp4)
    }

    pub(crate) fn from_mp4(mp4: &mut mp4iter::Mp4) -> Result<Self, GpmfError> {
        let mut firm = mp4.find_user_data("FIRM")?;
        firm.read_to_string().map(|s| Self::from_firmware_id(&s))
            .map_err(|e| e.into())
    }

    pub(crate) fn from_meta(meta: &GoProMeta) -> Result<Self, GpmfError> {
        Ok(meta.device()?.name)
    }

    // pub fn from_firmware_id_old(id: &str) -> Self {
    //     match &id[..3] {
    //         "HD5" => Self::Hero5Black,
    //         "HD6" => Self::Hero6Black,
    //         "FS1" => Self::Fusion,
    //         "HD7" => Self::Hero7Black,
    //         "HD8" => Self::Hero8Black,
    //         "HD9" => Self::Hero9Black, // possibly H20
    //         "H19" => Self::Max,
    //         "H20" => Self::Hero9Black, // possibly HD9, and H20 is another device
    //         "H21" => Self::Hero10Black,
    //         "H22" => Self::Hero11Black,
    //         "H23" => Self::Hero12Black,
    //         "H24" => Self::Hero13Black, // e.g. H24.01.01.12.00
    //         "H26" => Self::Mission1, // e.g. H24.01.01.12.00
    //         // Max2 id also starts with H24,
    //         // but second value differs:
    //         // H24.01 vs H24.02
    //         // "H24" => Self::Hero13Black, // e.g. H24.01.01.12.00
    //         _ => Self::Unknown
    //     }
    // }

    /// should use HXX.YY for model not just HXX:
    /// HXX.YY.AA.BB.CC =>
    /// model: H.XX.YY
    /// firmware version: AA.BB.CC
    /// see: https://github.com/nitrxgen/gopro-firmware
    pub fn from_firmware_id(id: &str) -> Self {
        match &id[..6] {
            "HD3.01" => Self::Hero3White,
            "HD3.02" => Self::Hero3Silver,
            "HD3.03" => Self::Hero3Black,
            "HD3.10" => Self::Hero3PlusSilver,
            "HD3.11" => Self::Hero3PlusBlack,
            "HD3.20" => Self::Hero2014,
            "HD3.21" => Self::HeroPlusLcd,
            "HD3.22" => Self::HeroPlus,
            "HD4.01" => Self::Hero4Silver,
            "HD4.02" => Self::Hero4Black,
            "HX1.01" => Self::HeroSession,
            "HD5.02" => Self::Hero5Black,
            "HD5.03" => Self::Hero5Session,
            "HD6.01" => Self::Hero6Black,
            "FS1.04" => Self::Fusion,
            "HD7.01" => Self::Hero7Black,
            "HD8.01" => Self::Hero8Black,
            "HD9.01" => Self::Hero9Black,
            "H18.01" => Self::Hero2018,
            "H18.02" => Self::Hero7White,
            "H18.03" => Self::Hero7Silver,
            "H19.03" => Self::Max,
            // "H20" => Self::Hero9Black, // possibly HD9, and H20 is another device
            "H9.01" => Self::Hero9Black, // HD9 according to https://github.com/nitrxgen/gopro-firmware
            "H21.01" => Self::Hero10Black,
            "H22.01" => Self::Hero11Black,
            "H22.03" => Self::Hero11BlackMini,
            "H23.01" => Self::Hero12Black,
            "H24.01" => Self::Hero13Black, // e.g. H24.01.01.12.00
            "H24.02" => Self::Max2,
            "H24.03" => Self::Hero2024,
            "H25.03" => Self::HeroLit,
            "H26.01" => Self::Mission1Pro, // e.g. H26.01.01.09.45
            "H26.02" => Self::Mission1, // e.g. H26.01.01.09.45
            // Max2 id also starts with H24,
            // but second value differs:
            // H24.01 vs H24.02
            // "H24" => Self::Hero13Black, // e.g. H24.01.01.12.00
            _ => Self::Unknown
        }
    }

    pub fn from_str(model: &str) -> Self {
        match model.trim().replace(" ", "").to_lowercase().as_str() {
            // Hero5 Black identifies itself as "Camera" so far.
            // "Camera" | "Hero5 Black" | "HERO5 Black" => Self::Hero5Black,
            // "Hero6 Black" | "HERO6 Black" => Self::Hero6Black,
            // "Hero7 Black" | "HERO7 Black" => Self::Hero7Black,
            // "Hero8 Black" | "HERO8 Black" => Self::Hero8Black,
            // "Hero9 Black" | "HERO9 Black" => Self::Hero9Black,
            // "Hero10 Black" | "HERO10 Black" => Self::Hero10Black,
            // "Hero11 Black" | "HERO11 Black" => Self::Hero11Black,
            // "Hero12 Black" | "HERO12 Black" => Self::Hero12Black,
            // "Hero13 Black" | "HERO13 Black" => Self::Hero13Black,
            // "MISSION 1 PRO" | "Mission 1 Pro" => Self::Mission1Pro,
            // "Fusion" | "FUSION" => Self::Fusion,
            // "GoPro Max" | "MAX" => Self::Max,
            // "GoPro Max2" | "MAX2" => Self::Max2,
            // "GoPro Karma v1.0" => Self::Karma,

            // // Hero5 Black identifies itself as "Camera" so far.
            // "camera" | "hero5 black" => Self::Hero5Black,
            // "hero6 black" => Self::Hero6Black,
            // "hero7 black" => Self::Hero7Black,
            // "hero8 black" => Self::Hero8Black,
            // "hero9 black" => Self::Hero9Black,
            // "hero10 black" => Self::Hero10Black,
            // "hero11 black" => Self::Hero11Black,
            // "hero12 black" => Self::Hero12Black,
            // "hero13 black" => Self::Hero13Black,
            // "mission 1 pro" => Self::Mission1Pro,
            // "fusion" => Self::Fusion,
            // "gopro max" | "max" => Self::Max,
            // "gopro max2" | "max2" => Self::Max2,
            // "gopro karma v1.0" => Self::Karma,
            // Hero5 Black identifies itself as "Camera" so far.
            "camera" | "hero5black" => Self::Hero5Black,
            "hero6black" => Self::Hero6Black,
            "hero7black" => Self::Hero7Black,
            "hero8black" => Self::Hero8Black,
            "hero9black" => Self::Hero9Black,
            "hero10black" => Self::Hero10Black,
            "hero11black" => Self::Hero11Black,
            "hero12black" => Self::Hero12Black,
            "hero13black" => Self::Hero13Black,
            "mission1" => Self::Mission1,
            "mission1pro" => Self::Mission1Pro,
            "fusion" => Self::Fusion,
            "gopromax" | "max" => Self::Max,
            "gopromax2" | "max2" => Self::Max2,
            "goprokarmav1.0" => Self::Karma,
            _ => Self::Unknown
        }
    }

    pub fn to_str(&self) -> &str {
        match self {
            Self::Hero2014 => "Hero (2014)",
            Self::Hero2018 => "Hero (2018)",
            Self::Hero3PlusBlack => "Hero3+ Black",
            Self::Hero3PlusSilver => "Hero3+ Silver",
            Self::Hero3Black => "Hero3 Black",
            Self::Hero3Silver => "Hero3 Silver",
            Self::Hero3White => "Hero3 White",
            Self::Hero4Black => "Hero4 Black",
            Self::Hero4Silver => "Hero4 Silver",
            Self::HeroSession => "Hero Session",
            Self::HeroPlus => "Hero+",
            Self::HeroPlusLcd => "Hero+ LCD",
            Self::Hero2024 => "Hero (2024)",
            Self::HeroLit => "Lit Hero",
            Self::Hero5Black => "Hero5 Black",
            Self::Hero5Session => "Hero5 Session",
            Self::Hero6Black => "Hero6 Black",
            Self::Hero7Black => "Hero7 Black",
            Self::Hero7White => "Hero7 White",
            Self::Hero7Silver => "Hero7 Silver",
            Self::Hero8Black => "Hero8 Black",
            Self::Hero9Black => "Hero9 Black",
            Self::Hero10Black => "Hero10 Black",
            Self::Hero11Black => "Hero11 Black",
            Self::Hero11BlackMini => "Hero11 Black Mini",
            Self::Hero12Black => "Hero12 Black",
            Self::Hero13Black => "Hero13 Black",
            Self::Mission1 => "Mission 1",
            Self::Mission1Pro => "Mission 1 Pro",
            Self::Fusion => "Fusion",
            Self::Max => "Max",
            Self::Max2 => "Max2",
            Self::Karma => "GoPro Karma v1.0", // only v1.0 so far
            Self::Unknown => "Unknown",
        }
    }

    // Get documented sample frequency for a specific device
    // pub fn freq(&self, fourcc: FourCC) {
    //     match self {

    //     }
    // }
}
