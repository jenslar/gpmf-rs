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
        // if let Ok(Some(hdr)) = mp4.find("mdat") {
        //     let cursor = mp4.read_at(hdr.data_offset(), 4)?;
        //     let fourcc = mp4iter::FourCC::from_slice(&cursor.into_inner());
        //     // For all GoPro cameras "GOPR" fourcc follows
        //     // immediately after header for mdat so far
        //     if fourcc == mp4iter::FourCC::Custom("GPRO".to_owned()) {
        //         mp4.seek(4)?; // seek past size for now to use first three chars from firmware (udta FIRM, udta gpmf FMWR)
        //         let id: String = mp4.read(3)?.into_inner().iter().map(|n| *n as char).collect();
        //         // return Ok(Self::from_firmware_id(&id))
        //         match Self::from_firmware_id(&id) {
        //             Some(dvnm) => return Ok(dvnm),
        //             None => return Err(GpmfError::UknownDevice)
        //         }
        //         // Size specified in little endian
        //         // let size = mp4.read_type_at::<u32>(4, hdr.data_offset(), binread::Endian::Little)?;
        //         // let size = mp4.read_type::<u32>(4, binread::Endian::Little)?; // seems correct
        //         // println!("SIZE: {size}");
        //         // let _gopr = mp4.read(size as u64)?; // cursor that shouldn't be more than 1500 bytes containing device name as ascii string
        //         // println!("GOPR: {_gopr:?}");
        //     }
        // }

        // Err(GpmfError::UknownDevice)

        // Ok(Self::Other(String::from("Unknown")))
    }

    pub(crate) fn from_file(mp4: &mut mp4iter::Mp4) -> Result<Self, GpmfError> {
        mp4.reset()?;
        if let Ok(Some(hdr)) = mp4.find("mdat") {
            let cursor = mp4.read_at(hdr.data_offset(), 4)?;
            let fourcc = mp4iter::FourCC::from_slice(&cursor.into_inner());
            // For all GoPro cameras "GOPR" fourcc follows
            // immediately after header for mdat so far
            if fourcc == mp4iter::FourCC::Custom("GPRO".to_owned()) {
                mp4.seek(4)?; // seek past size for now to use first three chars from firmware (udta FIRM, udta gpmf FMWR)
                let id: String = mp4.read(3)?.into_inner().iter().map(|n| *n as char).collect();
                // return Ok(Self::from_firmware_id(&id))
                match Self::from_firmware_id(&id) {
                    Some(dvnm) => return Ok(dvnm),
                    None => return Err(GpmfError::UknownDevice)
                }
                // Size specified in little endian
                // let size = mp4.read_type_at::<u32>(4, hdr.data_offset(), binread::Endian::Little)?;
                // let size = mp4.read_type::<u32>(4, binread::Endian::Little)?; // seems correct
                // println!("SIZE: {size}");
                // let _gopr = mp4.read(size as u64)?; // cursor that shouldn't be more than 1500 bytes containing device name as ascii string
                // println!("GOPR: {_gopr:?}");
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
            // _ => Self::Other("Unknown".to_owned())
        }
    }

    pub fn from_str(model: &str) -> Option<Self> {
        match model.trim() {
            // Hero5 Black identifies itself as "Camera"
            // inside GPMF so far.
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
            // s => Self::Other(s.to_owned()),
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
            // Self::Other(s) => s,
        }
    }

    // fn from_arr(arr: [u8; 4]) -> Self {
    //     match arr {
    //         b"Camera" => DeviceName::Hero5Black, // correct device name?
    //         b"Hero6 Black" | "HERO6 Black" => DeviceName::Hero6Black, // correct device name?
    //         b"Hero7 Black" | "HERO7 Black" => DeviceName::Hero7Black,
    //         b"Hero8 Black" | "HERO8 Black" => DeviceName::Hero8Black,
    //         b"Hero9 Black" | "HERO9 Black" => DeviceName::Hero9Black,
    //         b"Hero10 Black" | "HERO10 Black" => DeviceName::Hero10Black,
    //         b"Hero11 Black" | "HERO11 Black" => DeviceName::Hero11Black,
    //         b"Fusion" | "FUSION" => DeviceName::Fusion,
    //         b"GoPro Max" => DeviceName::GoProMax,
    //         b"GoPro Karma v1.0" => DeviceName::GoProKarma,
    //         s => DeviceName::Other(s.to_owned()),
    //     }
    // }

    // pub fn from_cursor(cursor: &mut Cursor<Vec<u8>>) -> Self {
    //     let mut bytes = [0_u8; 4];
    //     if let Ok(()) = cursor.read_exact(&mut bytes) {
            
    //         DeviceName::Other(String::from("Unknown"))
    //     } else {
    //         DeviceName::Other(String::from("Unknown"))
    //     }
    // }

    // Get documented sample frequency for a specific device
    // pub fn freq(&self, fourcc: FourCC) {
    //     match self {

    //     }
    // }
}
