//! GoPro "file", representing an original, unedited video clip of high and/or low resolution,
//! together with derived sequential number and other attributes.
//!
//! Structs for locating and working with MP4-files belonging to the same recording session.

use std::{path::{Path, PathBuf}, io::{Cursor, copy}};

use binread::{BinReaderExt, BinResult};
use blake3;
use mp4iter::{self, FourCC, Offset, Mp4};
use time::{Duration, PrimitiveDateTime, ext::NumericalDuration};

use crate::{
    GpmfError,
    Gpmf, DeviceName, files::fileext_to_lcstring,
};

use super::{GoProMeta, GoProFileType};

/// Represents an original, unedited GoPro MP4-file.
// #[derive(Debug, Clone, PartialEq, Eq, PartialOrd)] // TODO PartialOrd needed for Ord
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GoProFile {
    /// GoPro device name, use of e.g. MUID
    /// and present GPMF data may differ
    /// depending on model.
    pub device: DeviceName,
    /// High resolution MP4 (`.MP4`)
    pub mp4: Option<PathBuf>,
    // pub(crate) mp4_offsets: Vec<Offset>,
    /// Low resolution MP4 (`.LRV`)
    pub lrv: Option<PathBuf>,
    // pub(crate) lrv_offsets: Vec<Offset>,
    /// Media Unique ID.
    /// Used for matching MP4 and LRV clips,
    /// and recording sessions.
    /// Device dependent.
    /// - Hero11:
    ///     - MP4, LRV both have a value
    ///     - `MUID` matches for all clips in the same session.
    /// - Hero7:
    ///     - MP4 has a value
    ///     - LRV unknown
    ///     - `MUID` differs for all clips in the same session (use `GUMI`).
    pub muid: Vec<u32>,
    /// Global Unique ID.
    /// Used for matching MP4 and LRV clips,
    /// and recording sessions.
    /// Device dependent.
    /// 
    /// - Hero11:
    ///     - Multi-clip session:
    ///         - MP4 has a value
    ///         - LRV always set to `[0, 0, 0, ...]`
    ///         - `GUMI` differs for MP4 clips in the same session (use `MUID`)
    ///     - Single-clip session:
    ///         - MP4 has a value
    ///         - LRV has a value
    ///         - `GUMI` matches between MP4 and LRV
    /// - Hero7:
    ///     - Multi-clip session:
    ///         - MP4 has a value
    ///         - LRV unknown
    ///         - `GUMI` matches for clips in the same session (MP4)
    pub gumi: Vec<u8>,
    /// Fingerprint that is supposedly equivalent for
    /// high and low resolution video clips.
    /// Blake3 hash generated from concatenated `Vec<u8>`
    /// representing the full GPMF data (uninterpreted).
    pub fingerprint: Vec<u8>,
    // pub fingerprint: Option<Vec<u8>>
    pub creation_time: PrimitiveDateTime,
    pub duration: Duration,
    pub time_first_frame: Duration
}

// // TODO need to implement PartialOrd as well...
// impl Ord for GoProFile {
//     fn cmp(&self, other: &Self) -> std::cmp::Ordering {
//         // get self gpmf -> last timstamp
//         // get other gpmf -> first timstamp
//         // cmp: last timestamp < first timestamp

//         let ts_self = self.gpmf().map(|g| g.last_timestamp().cloned().unwrap_or_default());
//         let ts_other = other.gpmf().map(|g| g.first_timestamp().cloned().unwrap_or_default());

//         if let (Ok(t1), Ok(t2)) = (ts_self, ts_other) {
//             t2.cmp(&t1)
//         } else {
//             panic!("Failed to compare GoPro timestamps")
//         }
//     }
// }

impl GoProFile {
    // pub fn new(path: &Path, verify_gpmf: bool) -> Result<Self, GpmfError> {
    pub fn new(path: &Path) -> Result<Self, GpmfError> {
        let mut gopro = Self::default();
        
        let mut mp4 = mp4iter::Mp4::new(&path)?;


        gopro.time_first_frame = Self::time_first_frame(&mut mp4)?;
        mp4.reset()?;

        // Get GPMF DEVC byte offsets, duration, and sizes
        let offsets = mp4.offsets("GoPro MET")?;
        mp4.reset()?;

        let mp4_datetime = mp4.time()?;

        gopro.creation_time = mp4_datetime.0;
        gopro.duration = mp4_datetime.1;

        // Check if input passes setting device, MUID, GUMI...
        // Device derived from start of mdat
        gopro.device = Self::device_internal(&mut mp4)?;
        // MUID, GUMI determined from udta
        gopro.muid = Self::muid_internal(&mut mp4)?;
        gopro.gumi = Self::gumi_internal(&mut mp4)?;

        // ...and set path if ok
        let _filetype = gopro.set_path(path);
        // gopro.set_offsets(&mut mp4, filetype)?;

        // Set fingerprint as hash of raw GPMF byte stream
        // gopro.fingerprint = GoProFile::fingerprint(path)?;
        gopro.fingerprint = Self::fingerprint_internal_mp4(&mut mp4, offsets.first())?;

        Ok(gopro)
    }

    /// Time from midnight. Used for sorting clips.
    fn time_first_frame(mp4: &mut Mp4) -> Result<Duration, GpmfError> {
        mp4.reset()?;

        let tmcd = mp4.tmcd("GoPro TCD")?;

        let offset = tmcd.offsets.first()
            .ok_or_else(|| GpmfError::NoMp4Offsets)?;

        let unscaled_time = mp4.read_type_at::<u32>(offset.size as u64, offset.position, binread::Endian::Big)?;

        let duration = (unscaled_time as f64 / tmcd.number_of_frames as f64).seconds();
        
        Ok(duration)
    }

    /// Get video path.
    /// Prioritizes high-resolution video.
    pub fn path(&self) -> Option<&Path> {
        if self.mp4.is_some() {
            self.mp4.as_deref()
        } else {
            self.lrv.as_deref()
        }
    }

    pub fn filetype(path: &Path) -> Option<GoProFileType> {
        match fileext_to_lcstring(path).as_deref() {
            Some("mp4") => Some(GoProFileType::MP4),
            Some("lrv") => Some(GoProFileType::LRV),
            _ => None
        }
    }

    /// Calculates a Blake3 checksum from a `Vec<u8>`
    /// representing the concatenated GPMF byte streams.
    /// For use as clip identifier (as opposed to file),
    /// to determine which high (`.MP4`) and low-resolution (`.LRV`)
    /// clips that correspond to each other. The GPMF data should be
    /// identical for high and low resolution clips.
    pub fn fingerprint(path: &Path) -> Result<Vec<u8>, GpmfError> {
        // Determine Blake3 hash for Vec<u8>
        let mut cursor = Gpmf::first_raw(path)?;
        let mut hasher = blake3::Hasher::new();
        let _size = copy(&mut cursor, &mut hasher)?;
        let hash = hasher.finalize().as_bytes().to_ascii_lowercase();

        Ok(hash)
    }

    /// Calculates a Blake3 checksum from a `Vec<u8>`
    /// representing the first DEVC container.
    /// For use as clip identifier (as opposed to file),
    /// to determine which high (`.MP4`) and low-resolution (`.LRV`)
    /// clips that correspond to each other. The GPMF data should be
    /// identical for high and low resolution clips.
    fn fingerprint_internal_mp4(mp4: &mut mp4iter::Mp4, offset: Option<&Offset>) -> Result<Vec<u8>, GpmfError> {
        // mp4.reset()?;
        // let offsets = mp4.offsets("GoPro MET")?;
        
        // // Create a single Vec<u8> from GPMF streams
        // let mut bytes: Vec<u8> = Vec::new();
        // for offset in offsets.iter() {
        //     // Read and return data at MP4 offsets
        //     let cur = mp4.read_at(offset.position as u64, offset.size as u64)
        //         .map_err(|e| GpmfError::Mp4Error(e))?;
        //     bytes.extend(cur.into_inner()); // a bit dumb to get out of
        // }

        let mut cursor: Cursor<Vec<u8>>;
        if let Some(o) = offset {
            cursor = mp4.read_at(o.position, o.size as u64)?;
        } else {
            // Determine Blake3 hash for Vec<u8>
            cursor = Gpmf::first_raw_mp4(mp4)?;
        }
        let mut hasher = blake3::Hasher::new();
        let _size = copy(&mut cursor, &mut hasher)?;
        let hash = hasher.finalize().as_bytes().to_ascii_lowercase();

        Ok(hash)
    }

    pub fn fingerprint_hex(&self) -> String {
        self.fingerprint.iter()
            .map(|b| format!("{:02x}", b)) // pad single char hex with 0
            .collect::<Vec<_>>()
            .join("")
    }

    /// Set high or low-resolution path
    /// depending on file extention.
    pub fn set_path(&mut self, path: &Path) -> GoProFileType {
        match fileext_to_lcstring(path).as_deref() {
            Some("mp4") => {
                self.mp4 = Some(path.to_owned());
                GoProFileType::MP4
            },
            Some("lrv") => {
                self.lrv = Some(path.to_owned());
                GoProFileType::LRV
            },
            _ => GoProFileType::ANY
        }
    }

    /// Returns device name, e.g. `Hero11 Black`.
    fn device_internal(mp4: &mut mp4iter::Mp4) -> Result<DeviceName, GpmfError> {
        DeviceName::from_file(mp4)
    }

    /// Returns device name, e.g. `Hero11 Black`.
    pub fn device(path: &Path) -> Result<DeviceName, GpmfError> {
        DeviceName::from_path(path)
    }

    /// Returns an `mp4iter::Mp4` object for the specified filetype:
    /// - `GoProFileType::MP4` = high-resolution clip
    /// - `GoProFileType::LRV` = low-resolution clip
    /// - `GoProFileType::ANY` = either, prioritizing high-resolution clip
    pub fn mp4(&self, filetype: GoProFileType) -> Result<mp4iter::Mp4, std::io::Error> {
        let path = match filetype {
            GoProFileType::MP4 => self.mp4.as_ref().ok_or_else(|| GpmfError::PathNotSet)?,
            GoProFileType::LRV => self.lrv.as_ref().ok_or_else(|| GpmfError::PathNotSet)?,
            GoProFileType::ANY => self.path().ok_or_else(|| GpmfError::PathNotSet)?,
        };
        mp4iter::Mp4::new(&path)
    }

    /// Returns GPMF byte offsets as `Vec<mp4iter::offset::Offset>`
    /// for the specified filetype:
    /// high-res = `GoProFileType::MP4`, low-res = `GoProFileType::LRV`,
    /// either = `GoProFileType::ANY`.
    pub fn offsets(&self, filetype: GoProFileType) -> Result<Vec<Offset>, GpmfError> {
        // if (filetype == GoProFileType::MP4 || filetype == GoProFileType::ANY) && !self.mp4_offsets.is_empty() {
        //     println!("USING HI OFF");
        //     Ok(self.mp4_offsets.to_owned())
        // } else if (filetype == GoProFileType::LRV || filetype == GoProFileType::ANY) && !self.lrv_offsets.is_empty() {
        //     println!("USING LO OFF");
        //     Ok(self.lrv_offsets.to_owned())
        // } else {
        //     println!("DERIVE NEW OFF {filetype:?}");
        //     let mut mp4 = self.mp4(filetype)?;
        //     mp4.offsets("GoPro MET").map_err(|err| GpmfError::Mp4Error(err))
        // }
        let mut mp4 = self.mp4(filetype)?;
        mp4.offsets("GoPro MET").map_err(|err| GpmfError::Mp4Error(err))
    }

    // fn set_offsets(&mut self, mp4: &mut mp4iter::Mp4, filetype: GoProFileType) -> Result<(), GpmfError> {
    //     let offsets = mp4.offsets("GoPro MET")?;
    //     match filetype {
    //         GoProFileType::MP4 => self.mp4_offsets = offsets,
    //         GoProFileType::LRV => self.lrv_offsets = offsets,
    //         ft => return Err(GpmfError::InvalidGoProFileType(ft)),
    //     }
    //     Ok(())
    // }

    /// Returns embedded GPMF data.
    pub fn gpmf(&self) -> Result<Gpmf, GpmfError> {
        let path = self.path().ok_or_else(|| GpmfError::PathNotSet)?;
        Gpmf::new(path, false)
        // if let Some(path) = &self.path() {
        //     Gpmf::new(path, false)
        // } else {
        //     Err(GpmfError::PathNotSet)
        // }
    }

    /// Returns single GPMF chunk (`DEVC`)
    /// with `length` at specified `position` (byte offset).
    pub fn gpmf_at_offset(
        &self,
        mp4: &mut mp4iter::Mp4,
        position: u64,
        length: u64,
        filetype: &GoProFileType
    ) -> Result<Gpmf, GpmfError> {
        // let mut mp4 = self.mp4(filetype)?;
        let mut cursor = mp4.read_at(position, length)?; // !!! TODO change offset.size to u64
        
        Gpmf::from_cursor(&mut cursor, false)
    }

    /// Returns first DEVC stream only for embedded GPMF data.
    pub(crate) fn gpmf_first(&self) -> Result<Gpmf, GpmfError> {
        if let Some(path) = &self.path() {
            let mut cursor = Gpmf::first_raw(path)?;
            Gpmf::from_cursor(&mut cursor, false)
        } else {
            Err(GpmfError::PathNotSet)
        }
    }

    /// Extract custom data in MP4 `udta` container.
    /// GoPro stores some device settings and info here,
    /// including a mostly undocumented GPMF-stream.
    pub fn meta(&self) -> Result<GoProMeta, GpmfError> {
        if let Some(path) = &self.path() {
            GoProMeta::new(path, false)
        } else {
            Err(GpmfError::PathNotSet)
        }
    }

    /// Media Unique ID
    pub fn muid(path: &Path) -> Result<Vec<u32>, GpmfError> {
        let mut mp4 = mp4iter::Mp4::new(path)?;
        let udta = mp4.udta()?;
        let fourcc = FourCC::from_str("MUID");

        for field in udta.fields.iter() {
            if field.name == fourcc {
                let no_of_entries = match ((field.size - 8) % 4, (field.size - 8) / 4) {
                    (0, n) => n,
                    (_, n) => panic!("Failed to determine MUID: {n} length field is not 32-bit aligned")
                };

                let mut fld = field.to_owned();

                return (0..no_of_entries).into_iter()
                    .map(|_| fld.data.read_le::<u32>()) // read LE to match GPMF
                    .collect::<BinResult<Vec<u32>>>()
                    .map_err(|err| GpmfError::BinReadError(err))
            }
        }

        Err(GpmfError::NoMuid)
    }

    /// Media Unique ID
    fn muid_internal(mp4: &mut mp4iter::Mp4) -> Result<Vec<u32>, GpmfError> {
        mp4.reset()?;
        let udta = mp4.udta()?;
        let fourcc = FourCC::from_str("MUID");

        for field in udta.fields.iter() {
            if field.name == fourcc {
                let no_of_entries = match ((field.size - 8) % 4, (field.size - 8) / 4) {
                    (0, n) => n,
                    (_, n) => panic!("Failed to determine MUID: {n} length field is not 32-bit aligned")
                };

                let mut fld = field.to_owned();

                return (0..no_of_entries).into_iter()
                    .map(|_| fld.data.read_le::<u32>()) // read LE to match GPMF
                    .collect::<BinResult<Vec<u32>>>()
                    .map_err(|err| GpmfError::BinReadError(err))
            }
        }

        Err(GpmfError::NoMuid)
    }

    /// First four four digits of MUID.
    /// Panics if MUID contains fewer than four values.
    pub fn muid_first(&self) -> &[u32] {
        self.muid[..4].as_ref()
    }

    /// Last four digits of MUID.
    /// Panics if MUID contains fewer than eight values.
    pub fn muid_last(&self) -> &[u32] {
        self.muid[4..8].as_ref()
    }

    /// Global Unique Media ID
    pub fn gumi(path: &Path) -> Result<Vec<u8>, GpmfError> {
        // let meta = self.meta()?;
        let mut mp4 = mp4iter::Mp4::new(path)?;
        let udta = mp4.udta()?;
        let fourcc = FourCC::from_str("GUMI");

        for field in udta.fields.iter() {
            if field.name == fourcc {
                return Ok(field.to_owned().data.into_inner())
            }
        }

        Err(GpmfError::NoGumi)
    }
    /// Global Unique Media ID
    fn gumi_internal(mp4: &mut mp4iter::Mp4) -> Result<Vec<u8>, GpmfError> {
        mp4.reset()?;
        let udta = mp4.udta()?;
        let fourcc = FourCC::from_str("GUMI");

        for field in udta.fields.iter() {
            if field.name == fourcc {
                return Ok(field.to_owned().data.into_inner())
            }
        }

        Err(GpmfError::NoGumi)
    }

    pub fn time(&self) -> Result<(time::PrimitiveDateTime, time::Duration), GpmfError> {
        // LRV and MP4 paths will have identical duration so either is fine.
        let path = self.path().ok_or(GpmfError::PathNotSet)?;
        let mut mp4 = mp4iter::Mp4::new(&path)?;
        
        mp4.time().map_err(|err| GpmfError::Mp4Error(err))
    }

    /// Returns duration of clip.
    pub fn duration(&self) -> Result<Duration, GpmfError> {
        // LRV and MP4 paths will have identical duration so either is fine.
        let path = self.path().ok_or(GpmfError::PathNotSet)?;
        let mut mp4 = mp4iter::Mp4::new(&path)?;
        
        mp4.duration().map_err(|err| GpmfError::Mp4Error(err))
    }

    /// Returns duration of clip as milliseconds.
    pub fn duration_ms(&self) -> Result<i64, GpmfError> {
        self.duration()?
            .whole_milliseconds()
            .try_into()
            .map_err(|err| GpmfError::DowncastIntError(err))
    }
}

impl Default for GoProFile {
    fn default() -> Self {
        Self {
            device: DeviceName::default(),
            mp4: None,
            // mp4_offsets: Vec::default(),
            lrv: None,
            // lrv_offsets: Vec::default(),
            muid: Vec::default(),
            gumi: Vec::default(),
            fingerprint: Vec::default(),
            creation_time: mp4iter::time_zero(),
            duration: Duration::ZERO,
            time_first_frame: Duration::ZERO,
        }
    }
}