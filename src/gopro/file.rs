//! GoPro "file", representing an original, unedited video clip of high and/or low resolution,
//! together with identifiers `MUID` (Media Unique ID) and
//! `GUMI` (Global Unique ID) - both stored in the `udta` atom.
//!
//! A Blake3 hash of the first `DEVC` chunk is also calculated as a clip fingerprint/unique ID
//! that can be consistently used between models, since the use of `MUID` and `GUMI`, and
//! MP4 creation time is not.

use std::{
    io::{copy, SeekFrom},
    path::{Path, PathBuf}
};

use binrw::Endian;
use blake3;
use mp4iter::{Mp4, SampleOffset};
use time::{
    Duration,
    PrimitiveDateTime,
};

use crate::{
    files::fileext_to_lcstring,
    types::{Gumi, Muid},
    DeviceName,
    Gpmf,
    GpmfError,
    GOPRO_METADATA_HANDLER,
    GOPRO_MIN_WIDTH_HEIGHT,
    GOPRO_TIMECODE_HANDLER
};

use super::{GoProMeta, GoProFileType};

/// Represents an original, unedited GoPro MP4-file.
///
/// ## On unique clip identifiers
/// ### Media Unique ID (MUID).
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
///
/// ### Global Unique ID (GUMI).
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
    pub muid: Muid,
    /// Global Unique ID.
    /// Set to `[0, 0, 0, 0]` for the first
    /// low-resolution clip for some newer devices.
    pub gumi: Gumi,
    /// Fingerprint that is supposedly equivalent for
    /// high and low resolution video clips.
    /// Blake3 hash generated from the first GPMF data chunk,
    /// i.e. the first DEVC container, as raw bytes.
    pub fingerprint: Vec<u8>,
    pub(crate) creation_time: PrimitiveDateTime,
    pub(crate) duration: Duration,
    pub(crate) time_first_frame: Duration
}

// !!! faster to use muid/gumi etc for hashing to pair mp4 with lrv?
// !!! check which are identical for both (muid and gumi are not)
// impl Hash for GoProFile {
//     fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
//         self.device.hash(state);
//         self.mp4.hash(state);
//         self.lrv.hash(state);
//         self.muid.hash(state);
//         self.gumi.hash(state);
//         self.fingerprint.hash(state);
//         self.creation_time.hash(state);
//         self.duration.hash(state);
//         self.time_first_frame.hash(state);
//     }
// }

impl GoProFile {
    /// New `GoProFile` from path. Rejects any MP4 that is not
    /// an original, unedited GoPro MP4 clip.
    pub fn new(path: &Path) -> Result<Self, GpmfError> {
        let mut gopro = Self::default();

        let mut mp4 = mp4iter::Mp4::new(&path)?;

        // !!! should probably tweak order below
        // !!! and/or not reset to start all the time
        // !!! to avoid seeking back and forth in mp4

        gopro.time_first_frame = mp4.time_first_frame(false)?;

        // Get GPMF DEVC byte offsets, duration, and sizes
        // let offsets = mp4.offsets(&GOPRO_METADATA_HANDLER, true)?;
        let track_gpmf = mp4.track(GOPRO_METADATA_HANDLER, true)?;
        let first_offset = track_gpmf.offsets().nth(0).cloned();

        // Set MP4 time stamps
        let (creation_time, duration) = mp4.time(true)?;
        let (width, height) = mp4.resolution(false)?;
        gopro.creation_time = creation_time;
        gopro.duration = duration;

        // Check if input passes setting device, MUID, GUMI...
        // Device derived from start of mdat
        gopro.device = Self::device_internal(&mut mp4)?;

        // ...and set path if ok
        // let _filetype = gopro.set_path(path);
        // Attempt at filename independent high/low res clip check
        if (width, height) < GOPRO_MIN_WIDTH_HEIGHT {
            gopro.lrv = Some(path.to_path_buf())
        } else {
            gopro.mp4 = Some(path.to_path_buf())
        }

        // MUID, GUMI determined from udta
        gopro.muid = Self::muid_internal(&mut mp4)?;
        gopro.gumi = Self::gumi_internal(&mut mp4)?;

        // Set fingerprint as hash of first raw GPMF byte stream
        gopro.fingerprint = Self::fingerprint_internal_mp4(&mut mp4, first_offset.as_ref())?;

        Ok(gopro)
    }

    pub(crate) fn merge(&mut self, other: &GoProFile) -> Result<(), GpmfError>{
        // !!! more complete field comparison before release
        if self == other {
            // do nothing if files are equal
            return Ok(())
        }
        if self.fingerprint != other.fingerprint {
            return Err(GpmfError::FingerprintMismatch)
        }
        if other.gumi != [0, 0, 0, 0] {
            self.gumi = other.gumi
        }
        match (&other.mp4, &other.lrv) {
            (Some(hi), None) => self.mp4 = Some(hi.to_owned()),
            (None, Some(low)) => self.lrv = Some(low.to_owned()),
            (Some(hi), Some(low)) => {
                self.mp4 = Some(hi.to_owned());
                self.lrv = Some(low.to_owned());
            },
            _ => return Err(GpmfError::PathNotSet),
        }

        Ok(())
    }

    // /// Duration from midnight for the first frame. Used for sorting clips,
    // /// since logged creation time represents the start of the recording session,
    // /// not that of the clip for some GoPro devices.
    // fn time_first_frame(mp4: &mut Mp4) -> Result<Duration, GpmfError> {
    //     mp4.reset()?;
    //     let tmcd = mp4.tmcd(GOPRO_TIMECODE_HANDLER)?;

    //     let offset = tmcd.offsets.first()
    //         .ok_or_else(|| GpmfError::NoMp4Offsets(GOPRO_TIMECODE_HANDLER.to_owned()))?;

    //     // let unscaled_time = mp4.read_one::<u32>(Endian::Big, Some(offset.position))?;
    //     let unscaled_time = mp4.read_one::<u32>(true, Endian::Big, Some(offset.position))?;

    //     let duration = (unscaled_time as f64 / tmcd.number_of_frames as f64).seconds();

    //     Ok(duration)
    // }

    pub fn first_frame(&self) -> Duration {
        self.time_first_frame
    }

    /// Get video path.
    /// Prioritizes high-resolution video.
    pub fn path(&self) -> Result<&Path, GpmfError> {
        if self.mp4.is_some() {
            self.mp4.as_deref()
                .ok_or(GpmfError::HighResVideoNotSet)
        } else {
            self.lrv.as_deref()
                .ok_or(GpmfError::LowResVideoNotSet)
        }
    }

    pub fn filetype(path: &Path) -> Option<GoProFileType> {
        match fileext_to_lcstring(path).as_deref() {
            Some("mp4") => Some(GoProFileType::High),
            Some("lrv") => Some(GoProFileType::Low),
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
    fn fingerprint_internal_mp4(
        mp4: &mut mp4iter::Mp4,
        offset: Option<&SampleOffset>
    ) -> Result<Vec<u8>, GpmfError> {
        let mut cursor = match offset {
            // Some(o) => mp4.cursor_at(o.position, o.size as u64)?,
            // Some(o) => mp4.cursor(o.size as u64, Some(o.position))?,
            // Some(o) => mp4.cursor(&Source::File, o.size as u64, Some(o.position))?,
            Some(o) => mp4.cursor(o.size as u64, Some(SeekFrom::Start(o.position)))?,
            None => Gpmf::first_sample(mp4)?.into()
        };

        let mut hasher = blake3::Hasher::new();
        let _size = copy(&mut cursor, &mut hasher)?;
        let hash = hasher.finalize().as_bytes().to_ascii_lowercase();

        Ok(hash)
    }

    pub fn fingerprint_hex(&self) -> String {
        self.fingerprint.iter()
            .map(|b| format!("{:02x}", b)) // 0 padded, lower case hex
            .collect::<Vec<_>>()
            .join("")
    }

    /// Size of linked clip in bytes, preferring high-resolution (`.MP4`)
    /// over low-resolution (`.LRV`).
    pub fn size(&self) -> Result<u64, GpmfError> {
        Ok(self.path()?.metadata()?.len())
    }

    /// Size of high-resolution clip (`.MP4`) in bytes.
    pub fn size_mp4(&self) -> Result<u64, GpmfError> {
        Ok(self.mp4
            .as_deref()
            .ok_or(GpmfError::HighResVideoNotSet)?
            .metadata()?
            .len()
        )
    }

    /// Size of low-resolution clip (`.LRV`) in bytes.
    pub fn size_lrv(&self) -> Result<u64, GpmfError> {
        Ok(self.lrv
            .as_deref()
            .ok_or(GpmfError::LowResVideoNotSet)?
            .metadata()?
            .len()
        )
    }

    /// Set high or low-resolution path
    /// depending on file extention.
    pub fn set_path(&mut self, path: &Path) -> GoProFileType {
        match fileext_to_lcstring(path).as_deref() {
            Some("mp4") => {
                self.mp4 = Some(path.to_owned());
                GoProFileType::High
            },
            Some("lrv") => {
                self.lrv = Some(path.to_owned());
                GoProFileType::Low
            },
            _ => GoProFileType::Any
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

    /// Returns camera serial number, extracted
    /// from the `CAME` atom under `udta`.
    ///
    /// Should work all the way back to Hero5 Black,
    /// though some old sample videos report all zeroes
    /// for serial.
    pub fn serial(&self) -> Result<Vec<u8>, GpmfError> {
        // let mut serial = vec![0_u8; 16];
        // let path = self.path()?;
        // let mut mp4 = Mp4::new(path)?;
        // let mut atom = mp4.find_user_data("CAME")?;
        // serial = atom.read_data()?;

        // Ok(serial)
        let path = self.path()?;
        let mut mp4 = Mp4::new(path)?;
        let mut atom = mp4.find_user_data("CAME")?;

        // serial is the atom's data load
        Ok(atom.read_data()?)
    }

    /// Returns an `mp4iter::Mp4` object for the specified filetype:
    /// - `GoProFileType::High` = high-resolution clip
    /// - `GoProFileType::Low` = low-resolution clip
    /// - `GoProFileType::Any` = either, prioritizing high-resolution clip
    pub fn mp4(&self, filetype: GoProFileType) -> Result<mp4iter::Mp4, GpmfError> {
        let path = match filetype {
            GoProFileType::High => self.mp4.as_ref().ok_or_else(|| GpmfError::PathNotSet)?,
            GoProFileType::Low => self.lrv.as_ref().ok_or_else(|| GpmfError::PathNotSet)?,
            GoProFileType::Any => self.path()?,
        };

        Ok(Mp4::new(&path)?)
    }

    // /// Returns GPMF byte offsets as `Vec<mp4iter::offset::Offset>`
    // /// for the specified filetype:
    // /// - high-res = `GoProFileType::High`
    // /// - low-res = `GoProFileType::Low`,
    // /// - either = `GoProFileType::Any`.
    // // pub fn offsets(&self, filetype: GoProFileType) -> Result<Vec<Offset>, GpmfError> {
    // pub fn offsets<'a>(&'a self, filetype: GoProFileType) -> Result<&'a [Offset], GpmfError> {
    //     let mut mp4 = self.mp4(filetype)?;
    //     // mp4.offsets("GoPro MET", true).map_err(|err| err.into()) // GpmfError::Mp4Error(err))
    //     // mp4.offsets("GoPro MET", true).map_err(|err| err.into()) // GpmfError::Mp4Error(err))
    //     let track = mp4.track(&GOPRO_METADATA_HANDLER, true)?;
    //     Ok(track.attributes.offsets())
    // }

    /// Returns embedded GPMF data.
    pub fn gpmf(&self) -> Result<Gpmf, GpmfError> {
        let path = self.path()?;
        Gpmf::new(path, false)
    }

    /// Returns single GPMF chunk (`DEVC`)
    /// with `length` at specified `position` (byte offset)
    /// in an MP4 file.
    pub fn gpmf_at_offset(
        &self,
        mp4: &mut mp4iter::Mp4,
        position: u64,
        length: u64,
        _filetype: &GoProFileType
    ) -> Result<Gpmf, GpmfError> {
        let mut cursor = mp4.cursor(length, Some(SeekFrom::Start(position)))?;
        Gpmf::from_cursor(&mut cursor, false)
    }

    /// Export GPMF uninterpreted.
    pub fn gpmf_raw(&self) -> Result<Vec<u8>, GpmfError> {
        let path = self.path()?;
        Gpmf::export_raw(path)
    }

    /// Extract custom data in MP4 `udta` container.
    /// GoPro stores some device settings and info here,
    /// including a mostly undocumented GPMF-stream.
    pub fn meta(&self) -> Result<GoProMeta, GpmfError> {
        let path = &self.path()?;
        GoProMeta::new(path, false)
    }

    /// Media Unique ID
    pub fn muid(path: &Path) -> Result<[u32; 8], GpmfError> {
        let mut mp4 = mp4iter::Mp4::new(path)?;
        Self::muid_internal(&mut mp4)
    }

    /// Media Unique ID
    fn muid_internal(mp4: &mut mp4iter::Mp4) -> Result<[u32; 8], GpmfError> {
        let mut muid_atom = mp4.find_user_data("MUID")?;
        let (min, max) = (muid_atom.min(), muid_atom.max());

        muid_atom.read_one::<[u32; 8]>(Endian::Big, None) // no bounds check...
            .map_err(|e| GpmfError::Mp4Error(e))
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

    /// Global Unique Media ID (`GUMI`)
    pub fn gumi(path: &Path) -> Result<[u32; 4], GpmfError> {
        let mut mp4 = mp4iter::Mp4::new(path)?;
        Self::gumi_internal(&mut mp4)
    }

    /// Global Unique Media ID, internal method
    // fn gumi_internal(mp4: &mut mp4iter::Mp4) -> Result<Vec<u8>, GpmfError> {
    fn gumi_internal(mp4: &mut mp4iter::Mp4) -> Result<[u32; 4], GpmfError> {
        let mut gumi_atom = mp4.find_user_data("GUMI")?;

        gumi_atom.read_one::<[u32; 4]>(Endian::Big, None)
            .map_err(|e| GpmfError::Mp4Error(e))
    }

    pub fn start(&self) -> PrimitiveDateTime {
        self.creation_time
    }

    pub fn end(&self) -> PrimitiveDateTime {
        self.creation_time + self.duration
    }

    /// Returns duration of clip.
    pub fn duration(&self) -> Duration {
        self.duration
    }

    /// Returns duration of clip as milliseconds.
    pub fn duration_ms(&self) -> i128 {
        self.duration
            .whole_milliseconds()
    }

    /// Returns `true` if `other` is part
    /// of the same recording session.
    pub fn matches(&self, other: &GoProFile) -> bool {
        if self.device != other.device {
            return false
        }
        match self.device {
            // Hero 11 (possibly 12) uses the same MUID for clips in the same session.
            DeviceName::Hero11Black => self.muid == other.muid,
            // Hero7 uses GUMI. Others unknown, GUMI is a pure guess, but seems to work.
            _ => self.gumi == other.gumi,
        }
    }
}

impl Default for GoProFile {
    fn default() -> Self {
        Self {
            device: DeviceName::default(),
            mp4: None,
            // muid_mp4: [0; 8],
            // gumi_mp4: [0; 4],
            lrv: None,
            // muid_lrv: [0; 8],
            // gumi_lrv: [0; 4],
            muid: [0; 8],
            gumi: [0; 4],
            fingerprint: Vec::default(),
            creation_time: mp4iter::mp4_time_zero(),
            duration: Duration::ZERO,
            time_first_frame: Duration::ZERO,
        }
    }
}
