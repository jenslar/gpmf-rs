use std::{
    collections::{HashMap, HashSet},
    path::{Path, PathBuf},
};

use log::{debug, info};
use mp4iter::Mp4Error;
use time::{Duration, PrimitiveDateTime};
use walkdir::WalkDir;

use crate::{
    DeviceInfo,
    GOPRO_DATETIME_DEFAULT,
    GOPRO_VALID_EXTENSIONS,
    Gpmf,
    GpmfError,
    Gps,
    Imu,
    ImuType,
    files::{filename_startswith, has_extension},
    gopro::file::GoProFile
};

#[derive(Debug, PartialEq)]
pub struct GoProSession{
    pub(crate) creation_time: PrimitiveDateTime,
    pub resolution: (u16, u16),
    pub(crate) id: Vec<u8>,
    pub(crate) files: Vec<GoProFile>,
}

#[cfg(feature = "gpx")]
impl TryFrom<GoProSession> for gpx::Gpx {
    type Error = GpmfError;

    /// Export GoProSession GPS log to GPX.
    /// Note that this exports all points,
    /// bad and good. It is usually better to
    /// first prune points with satellite lock level below
    /// and dilution of precision above
    /// a given threshold and use the `From<Gps> for gpx::Gpx`
    /// implementation instead.
    fn try_from(value: GoProSession) -> Result<Self, Self::Error> {
        Ok(value.gpmf()?.gps().to_gpx())
    }
}

impl GoProSession {
    pub(crate) fn init() -> GoProSession {
        Self {
            creation_time: GOPRO_DATETIME_DEFAULT,
            resolution: (0, 0),
            // id: String::default(),
            id: Vec::new(),
            files: Vec::new(),
        }
    }
    /// Create a session from a single clip.
    pub fn single(path: &Path) -> Result<Self, GpmfError> {
        let file = GoProFile::new(path)?;
        Ok(Self {
            creation_time: file.creation_time,
            resolution: file.resolution,
            id: file.session_id().to_vec(),
            files: vec![file],
        })
    }

    /// Add file to session. It will only be added
    /// if the session is either empty OR
    /// already contains files from the same
    /// recording session.
    pub fn add(&mut self, other: GoProFile) {
        if self.is_empty() || self.in_session(&other, true) {
            self.files.push(other);
        }
    }

    pub fn add_path(&mut self, video: &Path, sort: bool) -> Result<(), GpmfError> {
        let file = GoProFile::new(video)?;
        self.add_file(file, sort)
    }

    pub fn add_file(&mut self, file: GoProFile, sort: bool) -> Result<(), GpmfError> {
        if self.is_empty() {
            self.id = file.session_id().to_vec();
            self.resolution = file.resolution;
            self.creation_time = file.creation_time;
            self.files.push(file);
        } else if self.in_session(&file, true) {
            self.files.push(file);
        }
        if sort {
            self.sort();
        }
        Ok(())
    }

    pub fn get(&self, index: usize) -> Option<&GoProFile> {
        self.files.get(index)
    }

    /// Locates remaining clips in recording session
    /// and returns the session.
    ///
    /// Specified video does not have to be the first clip
    /// chronologically.
    ///
    /// Matches resolution. I.e. if a low-res clip (`.LRV`)
    /// is specified, only low-res clips will be considered.
    ///
    /// If `dir` = `None` the parent dir of `video`
    /// will be used as search location.
    ///
    /// `ignore_errors` = `true` will discard most errors
    /// when determining whether a video file is a GoPro
    /// file or not.
    /// Errors for MP4 files with no "GoPro MET" track
    /// will always be discarded as they are not relevant.
    pub fn locate(
        video: &Path,
        dir: Option<&Path>,
        ignore_errors: bool,
    ) -> Result<Self, GpmfError> {
        let dir = match dir {
            Some(d) => d,
            None => video.parent().ok_or(GpmfError::NoParentDir)?,
        };

        if !dir.exists() {
            return Err(std::io::Error::new(
                std::io::ErrorKind::NotFound,
                dir.display().to_string(),
            )
            .into());
        };

        let mut session = Self::single(video)?;

        let mut count = 1;
        for result in WalkDir::new(dir) {
            let path = match result {
                Ok(f) => f.path().to_owned(),
                // Ignore errors, since these are often due to lack of read permissions
                Err(_) => continue,
            };

            // Skip 'AppleDouble' files on macOS.
            // Seems to be quarantine files so far.
            if filename_startswith(&path, "._") {
                continue;
            }

            if has_extension(&path, GOPRO_VALID_EXTENSIONS).is_some() {
                info!("[{:4}] {}", count, path.display());
                count += 1;
                match GoProFile::new(&path) {
                    Ok(gp) => session.add(gp),
                    Err(err) => match err {
                        // Always continue on error due to no
                        // "GoPro MET" track since these can not
                        // be original GoPro files.
                        GpmfError::Mp4Error(Mp4Error::NoSuchTrack(_)) => continue,
                        _ => match ignore_errors {
                            true => continue,
                            false => return Err(err),
                        },
                    },
                }
            }
        }

        session.sort();

        Ok(session)
    }

    /// Derives and returns sessions for
    /// both high (`.MP4`) and low resolution (`.LRV`)
    /// clips as a tuple: `(HIGH_RES_SESSION, LOW_RES_SESSION)`.
    ///
    /// Specified video does not have to be the first clip
    /// chronologically and may be either a high or low resolution
    /// clip.
    ///
    /// Note that there is no way to tell whether clips
    /// are missing and if so which one is missing.
    pub fn locate_both(
        video: &Path,
        dir: Option<&Path>,
        ignore_errors: bool,
    ) -> Result<GoProMultiSession, GpmfError> {
        GoProMultiSession::locate(video, dir, ignore_errors)
    }

    /// Derives and returns all sessions
    /// located in input `dir`.
    pub fn locate_all(
        dir: &Path,
        ignore_errors: bool,
    ) -> Result<Vec<GoProMultiSession>, GpmfError> {
        GoProMultiSession::locate_all(dir, ignore_errors)
    }

    /// Returns `true` if `other` is part of the
    /// same recording session,
    /// and has not yet been added.
    fn in_session(&self, other: &GoProFile, skip_if_added: bool) -> bool {
        let mut in_session = self.files
            .iter()
            .inspect(|gp| {
                debug!(
                    "{}\n  SESSION ID: {:?}\n     CLIP ID: {:?}",
                    gp.source.display(),
                    gp.session_id(),
                    gp.file_id()
                )
            })
            .any(|gp| gp.in_session(other));

        if skip_if_added {
            in_session &= !self.contains(other);
        }

        in_session
    }

    // WIP: attempt to detect whether clips are missing
    // pub fn is_complete(&self) {
    //     // generate presumed (not exact) timeline:
    //     // assume all clips but the last have the same duration
    //     // first frame for each clip -> add duration ->

    //     // check that last clip has short timespan that those
    //     // preceding it, if not -> INCOMPLETE

    //     let mut duration_excl_last: Option<Duration> = None;
    //     // let len = self.len();
    //     for (i, file) in self.iter().enumerate() {
    //         println!("{:2}. {} - {} = {}",
    //             i+1,
    //             file.time_first_frame(),
    //             file.time_first_frame() + file.duration(),
    //             file.duration(),
    //         );
    //         if i == 0 {
    //             duration_excl_last = Some(file.duration());
    //         }
    //         if let Some(dur) = duration_excl_last {
    //             debug!(" -> {}", file.time_first_frame() - dur)
    //         }
    //     }
    // }

    pub fn is_low_res(&self) -> bool {
        self.first().map(|gp| gp.is_low_res()).unwrap_or(false)
    }

    /// Sort clips chronologically by `GoProFile::time_first_frame`.
    ///
    /// This is so far the only timestamp that is
    /// progressive across clips in the same session.
    /// MP4 creation time in `mvhd` atom will have the same
    /// date logged for all GoPro clips belonging to the same
    /// recording session (this may or may not depend
    /// on the specific model).
    pub fn sort(&mut self) {
        self.files.sort_by_key(|k| k.time_first_frame)
        // sorts by MP4 creation time which will be the same for some gopro devices
        // self.0.sort_by_key(|k| k.start())
    }

    pub fn first(&self) -> Option<&GoProFile> {
        self.files.first()
    }

    pub fn last(&self) -> Option<&GoProFile> {
        self.files.last()
    }

    /// Returns `true` if GPS data is encountered,
    /// GPS5 or GPS9.
    pub fn has_gps(&self) -> Result<bool, GpmfError> {
        if let Some(first) = self.first() {
            return first.has_gps()
        }
        Ok(false)
    }

    pub fn basename(&self) -> Option<String> {
        match self.len() {
            0 => None,
            1 => self.first()?.basename()?.to_str().map(String::from),
            _ => Some(format!("{}-{}",
                self.first()?.basename()?.to_str()?,
                self.last()?.basename()?.to_str()?,
            )),
        }
    }

    pub fn is_empty(&self) -> bool {
        self.files.is_empty()
    }

    pub fn len(&self) -> usize {
        self.files.len()
    }

    pub fn iter(&self) -> impl Iterator<Item = &GoProFile> {
        self.files.iter()
    }

    pub fn iter_mut(&mut self) -> impl Iterator<Item = &mut GoProFile> {
        self.files.iter_mut()
    }

    pub fn contains(&self, other: &GoProFile) -> bool {
        // self.files.contains(other)
        self.files.iter().any(|f| f.file_id == other.file_id)
    }

    /// Returns device info: name and firmware version.
    pub fn device(&self) -> Option<&DeviceInfo> {
        self.first().map(|gp| &gp.device)
    }

    /// Returns paths to all files
    /// in session.
    pub fn paths(&self) -> impl Iterator<Item = &Path> {
        self.files.iter().map(|gp| gp.source.as_path())
    }

    pub fn files(&self) -> impl Iterator<Item = &GoProFile> {
        self.files.iter()
    }

    pub fn id(&self) -> &[u8] {
        &self.id
    }

    pub fn id_hex(&self) -> String {
        self.id.iter().map(|n| format!{"{n:02x}"}).collect()
    }

    /// Returns parent dir if all clips
    /// have the same parent dir.
    /// Returns `None` if they do not.
    pub fn dir(&self) -> Option<PathBuf> {
        let dir: HashSet<&Path> = self
            .paths()
            .filter_map(|p| p.parent())
            // .map(PathBuf::from)
            .collect();
        match dir.len() {
            1 => dir.iter().collect::<Vec<_>>().first().map(PathBuf::from),
            _ => None,
        }
    }

    /// Returns creation time for the session
    /// While this should becreation time for the first clip,
    /// all clips in the same session report the same
    /// creation time.
    pub fn creation_time(&self) -> Option<PrimitiveDateTime> {
        self.first().map(|gp| gp.creation_time)
    }

    /// Returns duration for the session.
    pub fn duration(&self) -> Duration {
        self.iter().map(|gp| gp.duration).sum()
    }

    /// Returns resolution in pixels as a tuple
    /// `(WIDTH, HEIGHT)`.
    pub fn resolution(&self) -> Option<(u16, u16)> {
        self.first().map(|f| f.resolution)
    }

    /// Extracts and merges GPMF streams for
    /// files in session.
    pub fn gpmf(&self) -> Result<Gpmf, GpmfError> {
        let mut gpmf = Gpmf::default();
        gpmf.duration = Some(self.duration());
        gpmf.creation_time = self.creation_time();
        for file in self.iter() {
            gpmf.merge_mut(&mut file.gpmf()?);
        }
        Ok(gpmf)
    }

    /// Extracts the GPS log.
    ///
    /// Reads from disk. I.e. if you
    /// need to extract multiple
    /// kinds of data within the same
    /// process it is probably more efficient
    /// to first extract the merged GPMF stream
    /// (`GoProSession::gpmf()` method),
    /// then run the various GPS
    /// extraction methods on that.
    pub fn gps(&self) -> Result<Gps, GpmfError> {
        Ok(self.gpmf()?.gps())
    }

    /// Extracts accelerometer data.
    ///
    /// Reads from disk. I.e. if you
    /// need to extract multiple
    /// kinds of data within the same
    /// process it is probably more efficient
    /// to first extract the merged GPMF stream
    /// (`GoProSession::gpmf()` method),
    /// then run the various sensor
    /// extraction methods on that.
    pub fn accelerometer(&self) -> Result<Vec<Imu>, GpmfError> {
        Ok(self.gpmf()?.imu(&ImuType::Accelerometer))
    }

    /// Extracts gyroscope data.
    ///
    /// Reads from disk. I.e. if you
    /// need to extract multiple
    /// kinds of data within the same
    /// process it is probably more efficient
    /// to first extract the merged GPMF stream
    /// (`GoProSession::gpmf()` method),
    /// then run the various sensor
    /// extraction methods on that.
    pub fn gyroscope(&self) -> Result<Vec<Imu>, GpmfError> {
        Ok(self.gpmf()?.imu(&ImuType::Gyroscope))
    }

    /// Extracts gravity vector data.
    ///
    /// Each combined point
    /// represents a normalised vector directed
    /// towards the gravity center of the earth.
    ///
    /// Reads from disk. I.e. if you
    /// need to extract multiple
    /// kinds of data within the same
    /// process it is probably more efficient
    /// to first extract the merged GPMF stream
    /// (`GoProSession::gpmf()` method),
    /// then run the various sensor
    /// extraction methods on that.
    pub fn gravity(&self) -> Result<Vec<Imu>, GpmfError> {
        Ok(self.gpmf()?.imu(&ImuType::GravityVector))
    }
}

/// Contains both high and low res session.
/// `id` corresponds to either `MUID` or `GUMI`
/// depending on model.
#[derive(Debug)]
pub struct GoProMultiSession {
    /// Session ID.
    /// Derived from Either MUID, GUMI, or CPID,
    /// depending on model.
    pub(crate) id: Vec<u8>,
    /// High-res clips
    pub(crate) high: GoProSession,
    /// Low-res clips
    pub(crate) low: GoProSession,
}

impl Default for GoProMultiSession {
    fn default() -> Self {
        Self {
            id: Default::default(),
            high: GoProSession::init(),
            low: GoProSession::init()
        }
    }
}

#[cfg(feature = "gpx")]
impl TryFrom<GoProMultiSession> for gpx::Gpx {
    type Error = GpmfError;

    /// Export GoProMultiSession GPS log to GPX.
    /// Note that this exports all points,
    /// bad and good. It is usually better to
    /// first prune points with satellite lock level below
    /// and dilution of precision above
    /// a given threshold and use the `From<Gps> for gpx::Gpx`
    /// implementation instead.
    fn try_from(value: GoProMultiSession) -> Result<Self, Self::Error> {
        Ok(value.gpmf()?.gps().to_gpx())
    }
}

impl GoProMultiSession {
    pub(crate) fn init(file: GoProFile) -> Result<Self, GpmfError> {
        let mut multi = Self::default();
        multi.id = file.session_id().to_vec();
            // .ok_or(GpmfError::NoSessionId)?;
        multi.add_file(file, false)?;
        Ok(multi)
    }

    pub fn add_path(&mut self, video: &Path, sort: bool) -> Result<(), GpmfError> {
        let file = GoProFile::new(video)?;
        self.add_file(file, sort)
    }

    pub fn add_file(&mut self, file: GoProFile, sort: bool) -> Result<(), GpmfError> {
        if &self.id == &file.session_id() {
            match file.is_low_res() {
                true => self.low.add_file(file, sort)?,
                false => self.high.add_file(file, sort)?,
            }
        }
        Ok(())
    }

    pub fn single(video: &Path) -> Result<GoProMultiSession, GpmfError> {
        Self::init(GoProFile::new(video)?)
    }

    pub fn locate_all(dir: &Path, ignore_errors: bool) -> Result<Vec<Self>, GpmfError> {
        // k: session.id(), val: multi session
        let mut multisessions: HashMap<Vec<u8>, GoProMultiSession> = HashMap::new();

        if !dir.exists() {
            return Err(std::io::Error::new(
                std::io::ErrorKind::NotFound,
                dir.display().to_string(),
            )
            .into());
        };

        let mut count = 1;
        for result in WalkDir::new(dir) {
            let path = match result {
                Ok(f) => f.path().to_owned(),
                // Ignore errors, since these are often due to lack of read permissions
                Err(_) => continue,
            };

            // Skip 'AppleDouble' files on macOS.
            // Seems to be quarantine files so far.
            if filename_startswith(&path, "._") {
                continue;
            }

            if has_extension(&path, GOPRO_VALID_EXTENSIONS).is_some() {
                info!("[{:4}] {}", count, path.display());
                count += 1;
                match GoProFile::new(&path) {
                    Ok(gp) => {
                        // attempt att returning error when adding file to session...
                        let mut error_when_adding_file: Option<GpmfError> = None;
                        multisessions
                            .entry(gp.session_id().to_vec())
                            .and_modify(|multisession| if let Err(err) = multisession.add_file(gp.clone(), true) {
                                error_when_adding_file = Some(err);
                            })
                            .or_insert(GoProMultiSession::init(gp)?);
                        if let Some(err) = error_when_adding_file && !ignore_errors {
                            return Err(err)
                        }
                    }
                    Err(err) => match err {
                        // Always continue on error due to no
                        // "GoPro MET" track since these can not
                        // be original GoPro files.
                        GpmfError::Mp4Error(Mp4Error::NoSuchTrack(_)) => continue,
                        _ => match ignore_errors {
                            true => continue,
                            false => return Err(err),
                        },
                    },
                }
            };
        }

        // Sort all multi sessions (permormance...?)
        Ok(multisessions
            .drain()
            .map(|(_, session)| {
                let mut session = session;
                session.sort();
                session
            })
            .collect::<Vec<_>>())
    }

    pub fn locate(
        video: &Path,
        dir: Option<&Path>,
        ignore_errors: bool,
    ) -> Result<GoProMultiSession, GpmfError> {
        let dir = match dir {
            Some(d) => d,
            None => video.parent().ok_or(GpmfError::NoParentDir)?,
        };

        if !dir.exists() {
            return Err(std::io::Error::new(
                std::io::ErrorKind::NotFound,
                dir.display().to_string(),
            )
            .into());
        };

        let gp = GoProFile::new(video)?;
        let mut multi_session = GoProMultiSession::init(gp)?;

        let mut count = 1;
        for result in WalkDir::new(dir) {
            let path = match result {
                Ok(f) => f.path().to_owned(),
                // Ignore errors, since these are often due to lack of read permissions
                Err(_) => continue,
            };

            // Skip 'AppleDouble' files on macOS.
            // Seems to be quarantine files so far.
            if filename_startswith(&path, "._") {
                continue;
            }

            if has_extension(&path, GOPRO_VALID_EXTENSIONS).is_some() {
                info!("[{:4}] {}", count, path.display());
                count += 1;
                match GoProFile::new(&path) {
                    Ok(gp) => {
                        multi_session.add_file(gp, false)?
                    },
                    Err(err) => match err {
                        // Always continue on error due to no
                        // "GoPro MET" track since these can not
                        // be original GoPro files.
                        GpmfError::Mp4Error(Mp4Error::NoSuchTrack(_)) => continue,
                        _ => match ignore_errors {
                            true => continue,
                            false => return Err(err),
                        },
                    },
                }
            }
        }

        multi_session.sort();

        Ok(multi_session)
    }

    pub fn is_empty(&self) -> bool {
        self.id.is_empty()
        && self.high.is_empty()
        && self.low.is_empty()
    }

    pub fn len(&self) -> usize {
        assert!(self.high.len() == self.low.len(), "High and low resolution vary in number of clips");
        self.high.len()
    }

    /// Sort sessions on time of first frmae
    /// which is so far the only incremental
    /// start time stamp between clips
    /// that belong in the same recording session.
    pub fn sort(&mut self) {
        self.high.sort();
        self.low.sort();
    }

    pub fn high(&self) -> &GoProSession {
        &self.high
    }

    pub fn low(&self) -> &GoProSession {
        &self.low
    }

    pub fn paths_high(&self) -> impl Iterator<Item = &Path> {
        self.high.paths()
    }

    pub fn paths_low(&self) -> impl Iterator<Item = &Path> {
        self.low.paths()
    }

    pub fn device(&self) -> Option<&DeviceInfo> {
        self.high.device().or_else(|| self.low.device())
    }

    pub fn creation_time(&self) -> Option<PrimitiveDateTime> {
        self.high.creation_time().or_else(|| self.low.creation_time())
    }

    pub fn id(&self) -> &[u8] {
        &self.id
    }

    pub fn id_hex(&self) -> String {
        self.id.iter().map(|n| format!{"{n:02x}"}).collect()
    }

    pub fn duration(&self) -> Duration {
        if !self.high.is_empty() {
            self.high.duration()
        } else {
            self.low.duration()
        }
    }

    pub fn gpmf(&self) -> Result<Gpmf, GpmfError> {
        match (self.high.is_empty(), self.low.is_empty()) {
            (true, false) => self.low().gpmf(),
            (false, true) => self.high().gpmf(),
            (_, _) => Err(GpmfError::NoData),
        }
    }

    /// Extracts the GPS log.
    ///
    /// Reads from disk. I.e. if you
    /// need to extract multiple
    /// kinds of data within the same
    /// process it is probably more efficient
    /// to first extract the merged GPMF stream
    /// (`GoProSession::gpmf()` method),
    /// then run the various GPS
    /// extraction methods on that.
    pub fn gps(&self) -> Result<Gps, GpmfError> {
        Ok(self.gpmf()?.gps())
    }

    /// Extracts accelerometer data.
    ///
    /// Reads from disk. I.e. if you
    /// need to extract multiple
    /// kinds of data within the same
    /// process it is probably more efficient
    /// to first extract the merged GPMF stream
    /// (`GoProSession::gpmf()` method),
    /// then extract data from that.
    pub fn accelerometer(&self) -> Result<Vec<Imu>, GpmfError> {
        Ok(self.gpmf()?.imu(&ImuType::Accelerometer))
    }

    /// Extracts gyroscope data.
    ///
    /// Reads from disk. I.e. if you
    /// need to extract multiple
    /// kinds of data within the same
    /// process it is probably more efficient
    /// to first extract the merged GPMF stream
    /// (`GoProSession::gpmf()` method),
    /// then extract data from that.
    pub fn gyroscope(&self) -> Result<Vec<Imu>, GpmfError> {
        Ok(self.gpmf()?.imu(&ImuType::Gyroscope))
    }

    /// Extracts gravity vector data.
    ///
    /// Each combined point
    /// represents a normalised vector directed
    /// towards the gravity center of the earth.
    ///
    /// Reads from disk. I.e. if you
    /// need to extract multiple
    /// kinds of data within the same
    /// process it is probably more efficient
    /// to first extract the merged GPMF stream
    /// (`GoProSession::gpmf()` method),
    /// then extract data from that.
    pub fn gravity(&self) -> Result<Vec<Imu>, GpmfError> {
        Ok(self.gpmf()?.imu(&ImuType::GravityVector))
    }

    /// Returns an iterator over high and low res session files
    /// as a tuple `(HIGH_RES, LOW_RES)`.
    /// However, this means the iteration may stop early
    /// if clips didffer in number between the two.
    pub fn iter(&self) -> impl Iterator<Item = (&GoProFile, &GoProFile)> {
        self.high.iter().zip(self.low.iter())
    }

    pub fn has_gps(&self) -> Result<bool, GpmfError> {
        if let Some(first) = self.high().first().or(self.low().first()) {
            return first.has_gps()
        };
        Ok(false)
    }
}
