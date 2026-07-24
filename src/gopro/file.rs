//! GoPro "file", representing an original, unedited video clip of high and/or low resolution,
//! together with identifiers `MUID` (Media Unique ID) and
//! `GUMI` (Global Unique ID) - both stored in the `udta` atom.
//!
//! A Blake3 hash of the first `DEVC` chunk is also calculated as a clip fingerprint/unique ID
//! that can be consistently used between models, since the use of `MUID` and `GUMI`, and
//! MP4 creation time is not.

use std::{
    ffi::OsStr, io::copy, path::{Path, PathBuf}
};

use binrw::Endian;
use blake3;
use mp4iter::Mp4;
use time::{
    Duration,
    PrimitiveDateTime,
};

use crate::{
    Cpid,
    DeviceInfo,
    DeviceName,
    GOPRO_MIN_WIDTH_HEIGHT,
    Gpmf,
    GpmfError,
    Gps,
    Imu,
    ImuType,
    Stream,
    types::{Gumi, Muid},
};

use super::GoProMeta;

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
/// Used for matching clips in the
/// same recording sessions.
/// Device dependent.
///
/// - Hero7:
///     - Multi-clip session:
///         - MP4 has a value
///         - LRV unknown
///         - `GUMI` matches for clips in the same session (MP4)
/// - Hero11 and later:
///     - Multi-clip session:
///         - MP4 has a value
///         - LRV always set to `[0, 0, 0, ...]`
///         - `GUMI` differs for MP4 clips in the same session (use `MUID`)
///     - Single-clip session:
///         - MP4 has a value
///         - LRV has a value
///         - `GUMI` matches between MP4 and LRV
#[derive(Debug, Clone, PartialEq)]
pub struct GoProFile {
    /// GoPro device info, use of e.g. MUID
    /// and present GPMF data may differ
    /// depending on model.
    // pub device: DeviceName,
    pub device: DeviceInfo,
    /// Source video clip.
    pub source: PathBuf,
    /// Media Unique ID.
    pub(crate) muid: Muid,
    /// Global Unique ID.
    /// Set to `[0, 0, 0, 0]` for the first
    /// low-resolution clip for some newer devices.
    pub(crate) gumi: Gumi,
    /// Clip ID.
    /// Found in Hero 13 (and later?)
    /// GPMF section in the
    /// `udta` atom. Same CPID for
    /// all clips that belong to
    /// the same recording session so far.
    pub(crate) cpid: Option<Cpid>,
    /// Clip index.
    /// Found in Hero 13 (and later?)
    /// GPMF section in the
    /// `udta` atom. Denotes order
    /// in the recording session.
    pub(crate) cpin: Option<usize>,
    /// Blake3 hash generated from the first GPMF data chunk,
    /// i.e. the first DEVC container, as raw bytes.
    /// "Fingerprint" that is equivalent for
    /// high and low resolution video clips.
    pub(crate) file_id: Vec<u8>,
    /// Constructed either from MUID, GUMI, or CPID
    /// depending on model.
    pub(crate) session_id: Vec<u8>,
    /// MP4 creation time.
    /// This timestamp may be deceiving,
    /// since GoPro logs the same creation
    /// time for all clips that belong
    /// to the same recording session.
    /// Instead time of first frame is the better
    /// option, but
    pub(crate) creation_time: PrimitiveDateTime,
    /// MP4 duration.
    pub(crate) duration: Duration,
    /// MP4 timestamp first frame.
    pub(crate) time_first_frame: Duration,
    /// Video resolution in pixels. `(WIDTH, HEIGHT)`.
    /// Used to ensure matched clips have the same
    /// resolution (e.g. low-res/LRV clips are only matched
    /// with other low-res/LRV clips).
    pub(crate) resolution: (u16, u16),
    pub(crate) metadata: GoProMeta,
}

#[cfg(feature = "gpx")]
impl TryFrom<GoProFile> for gpx::Gpx {
    type Error = GpmfError;

    /// Export GoProFile GPS log to GPX.
    /// Note that this exports all points,
    /// bad and good. It is usually better to
    /// first prune points with satellite lock level below
    /// and dilution of precision above
    /// a given threshold and use the `From<Gps> for gpx::Gpx`
    /// implementation instead.
    fn try_from(value: GoProFile) -> Result<Self, Self::Error> {
        Ok(value.gpmf()?.gps().to_gpx())
    }
}

impl GoProFile {
    pub fn new(video: &Path) -> Result<Self, GpmfError> {
        let mut file = Self::default();

        let mut mp4 = Mp4::new(video)?;
        file.source = mp4.path().to_path_buf();

        // Create "fingerprint" (read from disk)
        // from hash of first raw DEVC chunk.
        file.file_id = Self::file_id_from_mp4(&mut mp4)?;

        // Get first frame timestamp (read from disk)
        // Exists in track data, not part of udta
        file.time_first_frame = mp4.time_first_frame(false)?;

        // Get additional data from udta atom (in-memory buffer)
        file.resolution = mp4.resolution(true)?;
        // creation time is the same across all clips
        // in the same recording session
        let (creation_time, duration) = mp4.time(true)?;
        file.creation_time = creation_time;
        file.duration = duration;
        // Firmware/device name could probably be read from start
        // of mdat atom for max compatibility. Layout of mdat not clear.
        // Note that Hero5 does not have GPMF data in udta atom.
        file.metadata = GoProMeta::from_mp4(&mut mp4)?;
        file.device = file.metadata.device()?;
        file.muid = file.metadata.muid();
        file.gumi = file.metadata.gumi();
        file.cpid = file.metadata.cpid();
        file.cpin = file.metadata.cpin();

        // Set session ID, derived from MUID, GUMI, or CPID
        // depending on model.
        file.session_id = file.session_id_u8()
            .ok_or(GpmfError::NoSessionId)?;

        Ok(file)
    }

    pub fn path(&self) -> &Path {
        &self.source
    }

    pub fn resolution(&self) -> (u16, u16) {
        // Ok(Mp4::new(&self.source)?.resolution(false)?)
        self.resolution
    }

    /// Returns `true` if the clip
    /// has a lower resolution than
    /// the minimum one configurable
    /// (1920 * 1080).
    pub(crate) fn is_low_res(&self) -> bool {
        self.resolution < GOPRO_MIN_WIDTH_HEIGHT
    }

    /// Derived unique session ID.
    /// Returns hexadecimal string that corresponds to
    /// either `MUID`, `GUMI`, or `CPID` depending on how the specific model.
    /// Hero9 and earlier seem to use
    /// `GUMI` for identifying clips in the same recording session,
    /// whereas Hero10 and later seem to use `MUID`.
    /// Hero 13 (and later?) use `CPID` as
    /// session ID, then adds `CPID`,
    /// presumably clip index to specify
    /// its position in the recording session.
    /// Not verified due to lack of data.
    ///
    /// Returned ID should be identical for all clips in the same
    /// recording session.
    pub fn session_id_hex(&self) -> Option<String> {
        match self.device.name {
            // Hero 10 and later (?) use
            // the same MUID for clips in
            // the same session. Max2 as well?
            DeviceName::Hero10Black
            | DeviceName::Hero11Black
            | DeviceName::Hero12Black => Some(self.muid_as_bytes().iter().map(|n| format!{"{n:02x}"}).collect::<String>()),
            // CPID is equal for files in the same session.
            // Unknown whetherused for Hero 12 as well (no data).
            DeviceName::Hero13Black => self.cpid_as_bytes().map(|cpid| cpid.iter().map(|n| format!{"{n:02x}"}).collect::<String>()),
            // Hero7 uses GUMI. Others unknown, GUMI is a pure guess, but seems to work.
            // Seems Hero 13 uses GUMI as well.
            // MUID is the same for all clips from the same camera so far,
            // whereas GUMI changes.
            _ => Some(self.gumi_as_bytes().iter().map(|n| format!{"{n:02x}"}).collect::<String>()),
        }
    }

    /// Returns either `MUID`, `GUMI` or `CPID` as `Vec<u32>`.
    pub(crate) fn session_id_u32(&self) -> Option<Vec<u32>> {
        match self.device.name {
            // Hero 10 and later (?) use
            // the same MUID for clips in
            // the same session. Max2 as well?
            DeviceName::Hero10Black
            | DeviceName::Hero11Black
            | DeviceName::Hero12Black => Some(self.muid.to_vec()),
            DeviceName::Hero13Black => Some(self.cpid?.to_vec()),
            // Hero7 uses GUMI. Others unknown, GUMI is a pure guess, but seems to work.
            // Seems Hero 13 uses GUMI as well.
            // MUID is the same for all clips from the same camera so far,
            // whereas GUMI changes.
            _ => Some(self.gumi.to_vec()),
        }
    }

    /// Returns either `MUID`, `GUMI` or `CPID` as `Vec<u8>`.
    pub(crate) fn session_id_u8(&self) -> Option<Vec<u8>> {
        match self.device.name {
            // Hero 10 and later (?) use
            // the same MUID for clips in
            // the same session. Max2 as well?
            DeviceName::Hero10Black
            | DeviceName::Hero11Black
            | DeviceName::Hero12Black => Some(self.muid_as_bytes()),
            DeviceName::Hero13Black => Some(self.cpid_as_bytes()?),
            // Hero7 uses GUMI. Others unknown, GUMI is a pure guess, but seems to work.
            // Seems Hero 13 uses GUMI as well.
            // MUID is the same for all clips from the same camera so far,
            // whereas GUMI changes.
            _ => Some(self.gumi_as_bytes()),
        }
    }

    /// Returns device name, e.g. `Hero11 Black`.
    fn info(mp4: &mut Mp4, reset: bool) -> Result<DeviceInfo, GpmfError> {
        if reset {
            mp4.reset()?;
        }
        DeviceInfo::from_mp4(mp4)
    }

    pub fn file_id(&self) -> &[u8] {
        &self.file_id
    }

    pub fn file_index(&self) -> Option<usize> {
        self.cpin
    }

    pub fn session_id(&self) -> &[u8] {
        &self.session_id
    }


    pub fn cpid(&self) -> Option<[u32; 4]> {
        self.cpid
    }

    /// Extract CPID from GPMF section in `udta` atom.
    fn cpid_from_udta_gpmf(mp4: &mut Mp4) -> Result<Option<[u32; 4]>, GpmfError> {
        let meta = GoProMeta::from_mp4(mp4)?;
        Ok(meta.cpid())
    }

    /// Extract CPIN from GPMF section in `udta` atom.
    fn cpin_from_udta_gpmf(mp4: &mut Mp4) -> Result<Option<usize>, GpmfError> {
        let meta = GoProMeta::from_mp4(mp4)?;
        Ok(meta.cpin())
    }

    /// Converts CPID back to BE bytes.
    pub fn cpid_as_bytes(&self) -> Option<Vec<u8>> {
        Some(self
            .cpid?
            .iter()
            .flat_map(|n| n.to_be_bytes())
            .collect())
    }

    pub fn muid(&self) -> [u32; 8] {
        self.muid
    }

    /// Media Unique ID
    /// - Cameras before Hero 13: Only `udta` GPMF contains full MUID, it is truncated in `udta` section (last four digits are 0)
    /// - Cameras after Hero 13: even in udta gpmf section muid is truncated (last four digits are 0)
    fn muid_from_udta_raw(mp4: &mut Mp4) -> Result<[u32; 8], GpmfError> {
        let mut muid_atom = mp4.find_user_data("MUID")?;
        // let (min, max) = (muid_atom.min(), muid_atom.max());
        muid_atom.read_one::<[u32; 8]>(Endian::Big, None)
            .map_err(|e| GpmfError::Mp4Error(e))
    }

    /// Extract MUID from GPMF section in `udta` atom.
    fn muid_from_udta_gpmf(mp4: &mut Mp4) -> Result<[u32; 8], GpmfError> {
        let meta = GoProMeta::from_mp4(mp4)?;
        Ok(meta.muid())
    }

    /// First four four digits of MUID.
    /// Panics if MUID contains fewer than four values.
    fn muid_first(&self) -> &[u32] {
        self.muid[..4].as_ref()
    }


    /// Last four digits of MUID.
    /// Panics if MUID contains fewer than eight values.
    fn muid_last(&self) -> &[u32] {
        self.muid[4..8].as_ref()
    }


    /// Converts MUID back to BE bytes.
    pub fn muid_as_bytes(&self) -> Vec<u8> {
        self
            .muid
            .iter()
            .flat_map(|n| n.to_be_bytes())
            .collect()
    }

    pub fn gumi(&self) -> [u32; 4] {
        self.gumi
    }

    /// Global Unique Media ID from `udta` section.
    fn gumi_from_udta_raw(mp4: &mut Mp4) -> Result<[u32; 4], GpmfError> {
        let mut gumi_atom = mp4.find_user_data("GUMI")?;
        // let (min, max) = (gumi_atom.min(), gumi_atom.max());
        gumi_atom.read_one::<[u32; 4]>(Endian::Big, None)
            .map_err(|e| GpmfError::Mp4Error(e))
    }

    /// Converts GUMI back to BE bytes.
    pub fn gumi_as_bytes(&self) -> Vec<u8> {
        self
            .gumi
            .iter()
            .flat_map(|n| n.to_be_bytes())
            .collect()
    }

    pub fn creation_time(&self) -> PrimitiveDateTime {
        self.creation_time
    }

    pub fn time_first_frame(&self) -> Duration {
        self.time_first_frame
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

    /// Returns metadata in MP4 `udta` section.
    /// For models newer than Hero 5,
    /// this includes undocumented GPMF
    /// data.
    pub fn meta(&self) -> &GoProMeta {
        &self.metadata
    }

    /// Returns file stem.
    /// For generating export paths etc.
    pub fn basename(&self) -> Option<&OsStr> {
        self.path().file_stem()
    }

    /// Returns `true` if self and `other`
    /// are part of the same recording session.
    /// I.e. MUID or GUMI match (depending on exact
    /// model) and video resolution is the same.
    ///
    /// Note that this returns `true` for an identical
    /// file.
    pub fn in_session(&self, other: &Self) -> bool {
        // skip if different devices
        if self.device != other.device {
            return false
        }
        let resolution_match = self.resolution == other.resolution;
        let session_match = self.session_id() == other.session_id();

        session_match && resolution_match
    }

    /// Create a file ID, a unique fingerprint for the
    /// clip by hashing the first raw telemetry chunk
    /// (first DEVC container).
    ///
    /// Presumably unique enough to at least identify clips (e.g. pair high and low res-clips),
    /// since it contains accelerometer data etc.
    fn file_id_from_mp4(mp4: &mut Mp4) -> Result<Vec<u8>, GpmfError> {
        let mut sample = Gpmf::first_sample(mp4)?;
        let mut hasher = blake3::Hasher::new();
        let _size = copy(&mut sample, &mut hasher)?;
        let hash = hasher.finalize().as_bytes().to_ascii_lowercase();

        Ok(hash)
    }

    pub fn gpmf(&self) -> Result<Gpmf, GpmfError> {
        Gpmf::from_mp4(&self.source)
    }

    /// Returns `true` if GPS data is encountered,
    /// GPS5 or GPS9.
    /// Note that this reads from disk.
    pub fn has_gps(&self) -> Result<bool, GpmfError> {
        let mut mp4 = Mp4::new(&self.source)?;
        let mut sample = Gpmf::first_sample(&mut mp4)?;
        let len = sample.len();
        let streams = Stream::new(&mut sample, len)?;
        let gpmf = Gpmf::from_streams(&streams);
        Ok(gpmf.has_gps())
    }

    pub fn gps(&self) -> Result<Gps, GpmfError> {
        Ok(self.gpmf()?.gps())
    }
    /// Returns accelerometer.
    pub fn accelerometer(&self) -> Result<Vec<Imu>, GpmfError> {
        Ok(self
            .gpmf()?
            .imu(&ImuType::Accelerometer)
        )
    }
    /// Returns gyroscope.
    pub fn gyroscope(&self) -> Result<Vec<Imu>, GpmfError> {
        Ok(self
            .gpmf()?
            .imu(&ImuType::Gyroscope)
        )
    }
    /// Returns gravity vector.
    pub fn gravity(&self) -> Result<Vec<Imu>, GpmfError> {
        Ok(self
            .gpmf()?
            .imu(&ImuType::GravityVector)
        )
    }
}

impl Default for GoProFile {
    fn default() -> Self {
        Self {
            device: DeviceInfo::default(),
            source: Default::default(),
            muid: [0; 8],
            gumi: [0; 4],
            cpid: None,
            cpin: None,
            file_id: Vec::default(),
            session_id: Vec::default(),
            creation_time: mp4iter::default_time(),
            duration: Duration::ZERO,
            time_first_frame: Duration::ZERO,
            resolution: (u16::default(), u16::default()),
            metadata: GoProMeta::default()
        }
    }
}
