//! GoPro core GPMF struct and methods.
//!
//! Input:
//! - original, unedited GoPro MP4 clips
//! - raw GPMF "files" extracted via e.g. FFmpeg
//! - byte slices
//! - original, unedited GoPro JPEG files
//!
//! Content will vary between devices and data types.
//! Note that timing is derived directly from the MP4 container, meaning GPMF tracks
//! exported with FFmpeg will not have relative time stamps for each data cluster.
//!
//! ```rs
//! use gpmf_rs::Gpmf;
//! use std::path::Path;
//!
//! fn main() -> std::io::Result<()> {
//!     let path = Path::new("GOPRO_VIDEO.MP4");
//!     let gpmf = Gpmf::new(&path)?;
//!     Ok(())
//! }
//! ```

use std::collections::HashSet;
use std::fs::File;
use std::io::{Cursor, Read};
use std::ops::Deref;
use std::path::{Path, PathBuf};
use std::sync::mpsc::{self, Sender};
use std::thread;

use jpegiter::{Jpeg, JpegTag};
use mp4iter::{Mp4, Mp4Error, Sample};
use rayon::iter::IntoParallelRefMutIterator;
use rayon::{
    iter::ParallelBridge,
    prelude::{
        IntoParallelRefIterator,
        ParallelIterator,
    }
};
use time::macros::datetime;
use time::PrimitiveDateTime;

use super::{FourCC, Stream, Timestamp};
use crate::{gopro::Dvid, DataType, GoProPoint, GpmfError, Gps};
use crate::{DeviceName, SensorData, SensorType, StreamType, GOPRO_METADATA_HANDLER};

/// Core GPMF struct.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Gpmf {
    /// GPMF streams.
    pub streams: Vec<Stream>,
    /// Path/s to the GoPro MP4 source/s
    /// the GPMF data was extracted from.
    pub source: Vec<PathBuf>,
}

impl Gpmf {
    /// Extract and parse GPMF data from file.
    /// Either an unedited GoPro MP4-file,
    /// JPEG-file,
    /// or a "raw" GPMF-file (e.g. "GoPro MET" track extracted
    /// via FFmpeg or a similar tool).
    ///
    /// > Note: Relative timestamps for all data loads is exclusive
    /// > to MP4, since these are derived from MP4 timing.
    ///
    /// ```
    /// use gpmf_rs::Gpmf;
    /// use std::path::Path;
    ///
    /// fn main() -> std::io::Result<()> {
    ///     let path = Path::new("GOPRO_VIDEO.MP4");
    ///     let gpmf = Gpmf::new(&path, false)?;
    ///     Ok(())
    /// }
    /// ```
    pub fn new(path: &Path, debug: bool) -> Result<Self, GpmfError> {
        let ext = path
            .extension()
            .and_then(|e| e.to_str())
            .map(|s| s.to_lowercase())
            .ok_or_else(|| GpmfError::InvalidFileType(path.to_owned()))?;

        match ext.as_ref() {
            "mp4" | "lrv" => Self::from_mp4(path, debug),
            "jpg" | "jpeg" => Self::from_jpg(path, debug),
            // Possibly "raw" GPMF-file
            _ => Self::from_raw(path, Some(50_000_000), debug), // 50MB max read size ok...?
        }
    }

    /// Returns first DEVC stream only and without parsing.
    ///
    /// Presumed to be unique enough to use as a fingerprint
    /// without having to hash the entire GPMF stream or the
    /// MP4 file itself.
    ///
    /// Used for producing a hash that can be stored in a `GoProFile`
    /// struct to match high and low resolution clips, or duplicate ones.
    // pub(crate) fn first_raw(path: &Path) -> Result<Cursor<Vec<u8>>, GpmfError> {
    pub(crate) fn first_raw(path: &Path) -> Result<Sample, GpmfError> {
        let mut mp4 = mp4iter::Mp4::new(path)?;
        Self::first_sample(&mut mp4)
    }

    /// Extracts first DEVC stream unparsed as `Cursor<Vec<u8>>`.
    ///
    /// Used for producing a hash that can be stored in a `GoProFile`
    /// struct to match high and low resolution clips, or duplicate ones.
    // pub(crate) fn first_raw_mp4(mp4: &mut mp4iter::Mp4) -> Result<Cursor<Vec<u8>>, GpmfError> {
    pub(crate) fn first_sample(mp4: &mut mp4iter::Mp4) -> Result<Sample, GpmfError> {
        let mut track = mp4.track(GOPRO_METADATA_HANDLER, true)?;
        let first = match track.samples().nth(0) {
            Some(result) => result?,
            None => return Err(GpmfError::NoData),
        };
        Ok(first)
    }

    pub fn from_mp4_mpsc(path: &Path) -> Result<Self, GpmfError> {
        let mut mp4 = Mp4::new(path)?;
        let mut track = mp4.track(GOPRO_METADATA_HANDLER, false)?;
        let len = track.len();
        let samples = track.samples();

        // let (tx, rx): (Sender<Sample>, Receiver<Stream>) = mpsc::channel();
        let (tx, rx) = mpsc::channel();

        for result in samples {
            let tx_clone: Sender<Vec<Stream>> = tx.clone();
            let mut sample = result?;
            let len = sample.len();
            thread::spawn(move || {
                let stream = Stream::new(&mut sample, len, false).expect("Failed to parse gpmf");
                tx_clone.send(stream).expect("Failed to send gpmf stream");
            });
        }

        let mut streams: Vec<Stream> = Vec::new();
        for _ in 0..len {
            if let Ok(strm) = rx.recv() {
                streams.extend(strm);
            }
        }

        Ok(Self {
            streams,
            source: vec![path.to_path_buf()]
        })
    }

    /// Test with external spinning hard drive
    pub(crate) fn from_mp4_par(path: &Path, debug: bool) -> Result<Self, GpmfError> {
        // Rust's BufReader deafult buffer size = 8192, slightly above
        // current GPMF sample size (8000 or slightly less).
        let mut mp4 = Mp4::new(path)?;
        let mut track =  mp4.track(GOPRO_METADATA_HANDLER, false)?;

        // let now = Instant::now();
        let mut streams: Vec<Stream> = track
            .samples()
            // .enumerate() // use stream timestamp for sorting instead?
            .par_bridge() // does this help at all when io is the bottleneck, esp on spinning disks?
            .flat_map(|result| {
                match result {
                    Ok(mut sample) => {
                        let len = sample.len();
                        let ts = Timestamp::from(sample.time());
                        match Stream::new(&mut sample, len, debug) {
                            Ok(vec) => vec
                                .into_iter()
                                .map(|s| Ok(s.with_time(&ts)))
                                .collect::<Vec<Result<Stream, GpmfError>>>(),
                            Err(err) => vec![Err(err)],
                        }
                    },
                    Err(err) => vec![Err(GpmfError::from(err))],
                }
            })
            .collect::<Result<Vec<Stream>, GpmfError>>()?;

        // par bridge does not preserve input order, sort on timestamp (all should be populated and not None)
        streams.sort_by_key(|smp| smp.time.to_owned());

        Ok(Self {
            streams,
            source: vec![path.to_owned()],
        })
    }

    pub fn from_mp4(path: &Path, debug: bool) -> Result<Self, GpmfError> {
        // Rust's BufReader deafult buffer size = 8192, slightly above
        // current GPMF sample size (8000 or slightly less).
        let mut mp4 = Mp4::new(path)?;
        let mut track =  mp4.track(GOPRO_METADATA_HANDLER, false)?;

        let mut samples: Vec<Sample> = track
            .samples()
            .collect::<Result<Vec<Sample>, Mp4Error>>()?;
        let streams = samples
            .par_iter_mut() // does this help at all when io is the bottleneck, esp on spinning disks?
            .map(|sample| {
                let len = sample.len();
                let ts = Timestamp::from(sample.deref());
                let stream = Stream::new(sample, len, debug)?
                    .into_iter()
                    .map(|s| s.with_time(&ts))
                    .collect::<Vec<Stream>>();
                Ok(stream)
            })
            .collect::<Result<Vec<Vec<Stream>>, GpmfError>>()?
            // .into_par_iter()
            .into_iter()
            .flatten()
            .collect::<Vec<Stream>>();

        Ok(Self {
            streams,
            source: vec![path.to_owned()],
        })
    }

    pub fn export_raw(path: &Path) -> Result<Vec<u8>, GpmfError> {
        let mut mp4 = Mp4::new(path)?;
        let mut track = mp4.track(GOPRO_METADATA_HANDLER, false)?;

        Ok(track.samples()
            .map(|result| Ok(result?.raw().to_vec()))
            .collect::<Result<Vec<_>, GpmfError>>()?
            .into_iter()
            .flatten()
            .collect())
    }

    /// Returns the embedded GPMF stream in a GoPro photo, JPEG only.
    pub fn from_jpg(path: &Path, debug: bool) -> Result<Self, GpmfError> {
        // Find and extract EXIf chunk with GPMF
        let segment = Jpeg::new(path)?
            .find(&JpegTag::APP6)
            .map_err(|err| GpmfError::JpegError(err))?;

        if let Some(mut app6) = segment {
            app6.seek(6); // seek past `GoPro\null`
            let len = app6.data.get_ref().len();
            let stream = Stream::new(&mut app6.data, len, debug)?;
            return Ok(Self {
                streams: stream,
                source: vec![path.to_owned()],
            });
        } else {
            Err(GpmfError::InvalidFileType(path.to_owned()))
        }
    }

    /// Returns GPMF from a "raw" GPMF-file, i.e. a file that holds only
    /// GPMF data.
    /// E.g. the "GoPro MET" track extracted from a GoPro MP4 with FFMpeg.
    ///
    /// Since the file is read into a memory buffer, `max_size` can be specified
    /// to avoid large file reads.
    pub fn from_raw(path: &Path, max_size: Option<u64>, debug: bool) -> Result<Self, GpmfError> {
        let file_size = path.metadata()?.len();
        if let Some(max) = max_size {
            if file_size > max {
                return Err(GpmfError::MaxFileSizeExceeded {
                    max,
                    got: file_size,
                    path: path.to_owned()
                })
            }
        }
        let mut buf = vec![0_u8; file_size as usize];
        File::open(path)?.read_exact(&mut buf)?;
        let mut reader = Cursor::new(buf);
        let streams = Stream::new(&mut reader, file_size as usize, debug)?;

        Ok(Self {
            streams,
            source: vec![path.to_owned()],
        })
    }

    /// GPMF from `Cursor<Vec<u8>>`.
    pub fn from_cursor(cursor: &mut Cursor<Vec<u8>>, debug: bool) -> Result<Self, GpmfError> {
        let len = cursor.get_ref().len();
        Ok(Self {
            streams: Stream::new(cursor, len, debug)?,
            source: vec![],
        })
    }

    pub fn print(&self) {
        self.iter()
            .enumerate()
            .for_each(|(i, s)| s.print(Some(i + 1), Some(self.len())))
    }

    /// Returns number of `Streams`.
    pub fn len(&self) -> usize {
        self.streams.len()
    }

    pub fn iter(&self) -> impl Iterator<Item = &Stream> {
        self.streams.iter()
    }

    pub fn par_iter(&self) -> impl ParallelIterator<Item = &Stream> {
        self.streams.par_iter()
    }

    pub fn iter_mut(&mut self) -> impl Iterator<Item = &mut Stream> {
        self.streams.iter_mut()
    }

    pub fn into_iter(self) -> impl IntoIterator<Item = Stream> {
        self.streams.into_iter()
    }

    /// Returns first DEVC stream
    pub fn first(&self) -> Option<&Stream> {
        self.streams.first()
    }

    /// Returns last DEVC stream
    pub fn last(&self) -> Option<&Stream> {
        self.streams.last()
    }

    /// Find streams with specified FourCC.
    pub fn find(&self, fourcc: &FourCC) -> Option<&Stream> {
        for stream in self.iter() {
            if stream.fourcc() == fourcc {
                return Some(stream);
            }
            match &stream.streams {
                StreamType::Nested(s) => {
                    for strm in s.iter() {
                        strm.find(fourcc);
                    }
                }
                StreamType::Values(_) => return None,
            }
        }

        None
    }

    /// Append `Stream`s to `self.streams`.
    pub fn append(&mut self, streams: &mut Vec<Stream>) {
        self.streams.append(streams)
    }

    /// Extend `self.streams`.
    pub fn extend(&mut self, streams: &[Stream]) {
        self.streams.extend(streams.to_owned())
    }

    /// Merges two GPMF streams, returning the merged stream,
    /// leaving `self` untouched.
    /// Assumed that specified `gpmf` follows after
    /// `self` chronologically.
    pub fn merge(&self, gpmf: &Self) -> Self {
        let mut merged = self.to_owned();
        merged.merge_mut(&mut gpmf.to_owned());
        merged
    }

    /// Merges two GPMF streams in place.
    /// Assumed that specified `gpmf` follows after
    /// `self` chronologically.
    // pub fn merge_mut(&mut self, gpmf: &mut Self) {
    pub fn merge_mut(&mut self, gpmf: &mut Self) {
        if let Some(ts) = self.last_timestamp() {
            // uses final timestamp (relative + duration)
            // of previous gpmf to offset
            // all timestamps in the following gpmf
            gpmf.offset_time(&ts);
        }

        // append() is faster than extend() here so far
        // see: https://github.com/rust-lang/rust-clippy/issues/4321#issuecomment-929110184
        // self.extend(&gpmf.streams);
        self.append(&mut gpmf.streams);
        self.source.extend(gpmf.source.to_owned());
    }

    /// Filters direct child nodes based on `StreamType`. Not recursive.
    pub fn filter(&self, data_type: &DataType) -> Vec<Stream> {
        self.streams
            .par_iter()
            .flat_map(|s| s.filter(data_type))
            .collect()
    }

    /// Returns an iterator over filtered child nodes
    /// based on `StreamType`. Not recursive.
    pub fn filter_iter<'a>(&'a self, data_type: &'a DataType) -> impl Iterator<Item = Stream> + 'a {
        self.iter().flat_map(move |s| s.filter(data_type))
    }

    /// Returns a parallel iterator over filtered child nodes
    /// based on `StreamType`. Not recursive.
    pub fn filter_par_iter<'a>(&'a self, data_type: &'a DataType) -> impl ParallelIterator<Item = Stream> + 'a {
        self.par_iter().flat_map(move |s| s.filter(data_type))
    }

    /// Returns all unique free text stream descriptions, i.e. `STNM` data.
    /// The hierarchy is `DEVC` -> `STRM` -> `STNM`.
    pub fn types(&self) -> Vec<String> {
        let mut unique: HashSet<String> = HashSet::new();
        for devc in self.streams.iter() {
            devc.find_all(&FourCC::STRM)
                .iter()
                .filter_map(|s| s.name())
                .for_each(|n| _ = unique.insert(n));
        }
        let mut types = unique.into_iter().collect::<Vec<_>>();
        types.sort();
        types
    }

    /// Returns summed duration of MP4 sources (longest track).
    /// Raises error if sources are not MP4-files
    /// (e.g. if source is a raw `.gpmf` extracted via FFmpeg).
    pub fn duration(&self) -> Result<time::Duration, GpmfError> {
        let mut duration = time::Duration::ZERO;
        for path in self.source.iter() {
            duration += mp4iter::Mp4::new(path)?.duration(false)?; // no need to reset offset since new file
        }
        Ok(duration)
    }

    /// Returns summed duration of MP4 sources (longest track)
    /// as milliseconds.
    /// Raises error if sources are not MP4-files
    /// (e.g. if source is a raw `.gpmf` extracted via FFmpeg).
    pub fn duration_ms(&self) -> Result<i64, GpmfError> {
        Ok((self.duration()?.as_seconds_f64() * 1000.0) as i64)
    }

    /// Add time offset to all `DEVC` timestamps
    pub fn offset_time(&mut self, time: &Timestamp) {
        self.iter_mut()
            .for_each(|devc| devc.time = devc.time.to_owned().map(|t| t.add(time)))
    }

    /// Returns first `Timestamp` in GPMF stream.
    pub fn first_timestamp(&self) -> Option<&Timestamp> {
        self.first().and_then(|devc| devc.time.as_ref())
    }

    /// Returns last `Timestamp` in GPMF stream.
    pub fn last_timestamp(&self) -> Option<&Timestamp> {
        self.last().and_then(|devc| devc.time.as_ref())
    }

    /// Device name. Extracted from first `Stream`.
    /// The Karma drone has two streams:
    /// one for the for the attached camera,
    /// another for the drone itself.
    /// Attached bluetooth devices may also generate
    /// GPMF data. In both cases the camera is so far
    /// the first device listed.
    /// Hero5 Black (the first GPMF GoPro) identifies
    /// itself as `Camera`.
    pub fn device_name(&self) -> Vec<String> {
        let names_set: HashSet<String> = self
            .streams
            .iter()
            .filter_map(|s| s.device_name())
            .collect();

        let mut names = Vec::from_iter(names_set);
        names.sort();

        names
    }

    /// Device ID. Extracted from first `Stream`.
    pub fn device_id(&self) -> Option<Dvid> {
        self.streams.first().and_then(|s| s.device_id())
    }

    /// Returns `2020-01-01 0:00` as `time::PrimitiveDateTime`,
    /// the starting offset and earliest timestamp
    /// for all GPMF datetimes.
    pub fn basetime() -> PrimitiveDateTime {
        datetime!(2020-01-01 0:00)
    }

    /// Returns GPS log. Extracts data from either `GPS5`
    /// or `GPS9`, depending on device.
    ///
    /// Hero11 logs both `GPS5` and `GPS9`, but deprecates `GPS5`.
    /// Hero 13 and on only logs `GPS9` (Hero 12 does not have a GPS module).
    ///
    /// `GPS9` will return 10x the amount of points,
    /// since each individual point is timestamped together with values for
    /// satellite lock and dilution with precision (DOP).
    ///
    /// `GPS5` instead only logs time, satellite lock and DOP once for each cluster.
    pub fn gps(&self) -> Gps {
        let device = self.device_name().first().map(|s| DeviceName::from_str(s));
        match device {
            Some(DeviceName::Hero11Black)
            | Some(DeviceName::Hero13Black) => self.gps9(),
            _ => self.gps5()
        }
    }

    /// For `GPS5` models, Hero10 and earlier. Deprecated from Hero11
    /// and on. Hero11 logs both `GPS5` and`GPS9`. Later models
    /// with a GPS-module are expected to only use `GPS9`.
    ///
    /// Since `GPS5` only logs datetime, GPS fix, and DOP
    /// per point-cluster, a single average point is returned
    /// per `STRM`. This is a linear average, but
    /// should be accurate enough for the up to 18Hz log.
    ///
    /// Implementing a latitude dependent average
    /// is a future possibility.
    pub fn gps5(&self) -> Gps {
        Gps::new(self
            .filter_iter(&DataType::Gps5)
            .filter_map(|s| GoProPoint::from_gps5(&s))
            .collect::<Vec<_>>())
    }

    /// For `GPS9` models, Hero11 and later.
    ///
    /// Since the newer `GPS9` format logs datetime,
    /// GPS fix, and DOP per-point, all points are returned,
    /// which means larger amounts of data.
    // pub fn gps9(&self, set_delta: bool) -> Gps {
    pub fn gps9(&self) -> Gps {
        Gps::new(self
            .filter_iter(&DataType::Gps9)
            .filter_map(|s| GoProPoint::from_gps9(&s))
            .flatten()
            .collect::<Vec<_>>())
    }

    /// Sensor data. Note that sensors depend on model.
    /// All (?) GoPro cameras contain gyroscope and accelerometer.
    pub fn sensor(&self, sensor_type: &SensorType) -> Vec<SensorData> {
        SensorData::from_gpmf(self, sensor_type)
    }
}
