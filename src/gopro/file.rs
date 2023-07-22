//! GoPro "file", representing an original, unedited video clip of high and/or low resolution,
//! together with identifiers `MUID` (Media Unique ID) and
//! `GUMI` (Global Unique ID) - both stored in the `udta` atom.
//! 
//! A Blake3 hash of the first `DEVC` chunk is also calculated as a clip fingerprint/unique ID
//! that can be consistently used between models, since the use of `MUID` and `GUMI`, and
//! MP4 creation time is not.

use std::{path::{Path, PathBuf}, io::copy};

use binrw::{BinReaderExt, BinResult, Endian};
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

impl GoProFile {
    /// New `GoProFile` from path. Rejects any MP4 that is not
    /// an original, unedited GoPro MP4 clip.
    pub fn new(path: &Path) -> Result<Self, GpmfError> {
        let mut gopro = Self::default();
        
        let mut mp4 = mp4iter::Mp4::new(&path)?;

        gopro.time_first_frame = Self::time_first_frame(&mut mp4)?;

        // Get GPMF DEVC byte offsets, duration, and sizes
        let offsets = mp4.offsets("GoPro MET")?; // resets MP4 to 0

        // Set MP4 time stamps, used for ordering
        let (creation_time, duration) = mp4.time(true)?;
        gopro.creation_time = creation_time;
        gopro.duration = duration;

        // Check if input passes setting device, MUID, GUMI...
        // Device derived from start of mdat
        gopro.device = Self::device_internal(&mut mp4)?;

        // MUID, GUMI determined from udta
        gopro.muid = Self::muid_internal(&mut mp4)?;
        gopro.gumi = Self::gumi_internal(&mut mp4)?;

        // ...and set path if ok
        let _filetype = gopro.set_path(path);

        // Set fingerprint as hash of raw GPMF byte stream
        gopro.fingerprint = Self::fingerprint_internal_mp4(&mut mp4, offsets.first())?;

        Ok(gopro)
    }

    /// Time from midnight. Used for sorting clips.
    fn time_first_frame(mp4: &mut Mp4) -> Result<Duration, GpmfError> {
        mp4.reset()?;

        let tmcd = mp4.tmcd("GoPro TCD")?;

        let offset = tmcd.offsets.first()
            .ok_or_else(|| GpmfError::NoMp4Offsets)?;

        let unscaled_time = mp4.read_type_at::<u32>(offset.position, Endian::Big)?;

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
    /// representing the first GPMF streams (i.e. first `DEVC` container).
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
        let mut cursor = match offset {
            Some(o) => mp4.cursor_at(o.position, o.size as u64)?,
            None => Gpmf::first_raw_mp4(mp4)?
        };

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
    /// - high-res = `GoProFileType::MP4`
    /// - low-res = `GoProFileType::LRV`,
    /// - either = `GoProFileType::ANY`.
    pub fn offsets(&self, filetype: GoProFileType) -> Result<Vec<Offset>, GpmfError> {
        let mut mp4 = self.mp4(filetype)?;
        mp4.offsets("GoPro MET").map_err(|err| err.into()) // GpmfError::Mp4Error(err))
    }

    /// Returns embedded GPMF data.
    pub fn gpmf(&self) -> Result<Gpmf, GpmfError> {
        let path = self.path().ok_or_else(|| GpmfError::PathNotSet)?;
        Gpmf::new(path, false)
    }

    /// Returns single GPMF chunk (`DEVC`)
    /// with `length` at specified `position` (byte offset).
    pub fn gpmf_at_offset(
        &self,
        mp4: &mut mp4iter::Mp4,
        position: u64,
        length: u64,
        _filetype: &GoProFileType
    ) -> Result<Gpmf, GpmfError> {
        let mut cursor = mp4.cursor_at(position, length)?;
        Gpmf::from_cursor(&mut cursor, false)
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
        Self::muid_internal(&mut mp4)
    }

    /// Media Unique ID
    fn muid_internal(mp4: &mut mp4iter::Mp4) -> Result<Vec<u32>, GpmfError> {
        let udta = mp4.udta(true)?;
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
        let mut mp4 = mp4iter::Mp4::new(path)?;
        Self::gumi_internal(&mut mp4)
    }

    /// Global Unique Media ID, internal method
    fn gumi_internal(mp4: &mut mp4iter::Mp4) -> Result<Vec<u8>, GpmfError> {
        let udta = mp4.udta(true)?;
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
        
        mp4.time(false).map_err(|err| err.into())
    }

    /// Returns duration of clip.
    pub fn duration(&self) -> Result<Duration, GpmfError> {
        // LRV and MP4 paths will have identical duration so either is fine.
        let path = self.path().ok_or(GpmfError::PathNotSet)?;
        let mut mp4 = mp4iter::Mp4::new(&path)?;
        
        mp4.duration(false).map_err(|err| err.into())
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
            fingerprint: Vec::default(),
            creation_time: mp4iter::mp4_time_zero(),
            duration: Duration::ZERO,
            time_first_frame: Duration::ZERO,
        }
    }
}