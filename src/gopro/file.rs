//! GoPro "file", representing an original, unedited video clip of high and/or low resolution,
//! together with derived sequential number and other attributes.
//!
//! Structs for locating and working with MP4-files belonging to the same recording session.

use std::{path::{Path, PathBuf}, io::{Cursor, copy}};

use binread::{BinReaderExt, BinResult};
use blake3;
use mp4iter::{self, FourCC};
use time::Duration;

use crate::{
    GpmfError,
    Gpmf, DeviceName, files::fileext_to_lcstring,
};

use super::GoProMeta;

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
    /// Low resolution MP4 (`.LRV`)
    pub lrv: Option<PathBuf>,
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
    pub fn new(path: &Path) -> Result<Self, GpmfError> {
        let mut gopro = Self::default();
        
        let mut mp4 = mp4iter::Mp4::new(&path)?;

        // Check if input passes setting device, MUID, GUMI...
        gopro.device = Self::device_internal(&mut mp4)?;
        gopro.muid = Self::muid_internal(&mut mp4)?;
        gopro.gumi = Self::gumi_internal(&mut mp4)?;

        // ...and set path if ok
        gopro.set_path(path);

        // Set fingerprint as hash of raw GPMF byte stream
        // gopro.fingerprint = GoProFile::fingerprint(path)?;
        gopro.fingerprint = GoProFile::fingerprint_internal(&mut mp4)?;

        Ok(gopro)
    }

    /// Get video path.
    /// Prioritizes high-resolution video.
    pub fn path(&self) -> Option<PathBuf> {
        if self.mp4.is_some() {
            self.mp4.to_owned()
        } else {
            self.lrv.to_owned()
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
    pub fn fingerprint_internal(mp4: &mut mp4iter::Mp4) -> Result<Vec<u8>, GpmfError> {
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

        // Determine Blake3 hash for Vec<u8>
        let mut cursor = Gpmf::first_raw_mp4(mp4)?;
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
    // pub fn set_path(&mut self, path: &Path) -> Result<(), GpmfError> {
    pub fn set_path(&mut self, path: &Path) {
        match fileext_to_lcstring(path).as_deref() {
            Some("mp4") => self.mp4 = Some(path.to_owned()),
            Some("lrv") => self.lrv = Some(path.to_owned()),
            _ => ()
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

    /// Returns embedded GPMF data.
    pub fn gpmf(&self) -> Result<Gpmf, GpmfError> {
        if let Some(path) = &self.path() {
            Gpmf::new(path, false)
        } else {
            Err(GpmfError::PathNotSet)
        }
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

        Ok(Vec::new())
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

        Ok(Vec::new())
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

        Ok(Vec::new())
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

        Ok(Vec::new())
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
            lrv: None,
            muid: Vec::default(),
            gumi: Vec::default(),
            fingerprint: Vec::default()
        }
    }
}