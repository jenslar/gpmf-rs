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
use std::io::Cursor;
use std::path::{PathBuf, Path};

use jpegiter::{Jpeg, JpegTag};
use rayon::prelude::{
    IntoParallelRefMutIterator,
    IndexedParallelIterator,
    IntoParallelRefIterator,
    ParallelIterator
};

use super::{FourCC, Timestamp, Stream};
use crate::{StreamType, SensorType, SensorData};
use crate::{
    Gps,
    GoProPoint,
    DataType,
    GpmfError,
    gopro::Dvid
};

/// Core GPMF struct.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Gpmf {
    /// GPMF streams.
    pub streams: Vec<Stream>,
    /// Path/s to the GoPro MP4 source/s
    /// the GPMF data was extracted from.
    pub source: Vec<PathBuf>
}

// impl From<dyn AsMut<Cursor<Vec<u8>>>> for Gpmf {
//     fn from<C: AsMut<Cursor<Vec<u8>>>>(value: C) -> Result<Self, GpmfError> {
//         Gpmf::from_cursor(value, false)
//     }
// }

impl Gpmf {
    /// GPMF from file. Either an unedited GoPro MP4-file,
    /// JPEG-file (WIP, currently n/a),
    /// or a "raw" GPMF-file, extracted via FFmpeg.
    /// Relative timestamps for all data loads is exclusive
    /// to MP4, since these are derived from MP4 timing.
    /// 
    /// ```
    /// use gpmf_rs::Gpmf;
    /// use std::path::Path;
    /// 
    /// fn main() -> std::io::Result<()> {
    ///     let path = Path::new("GOPRO_VIDEO.MP4");
    ///     let gpmf = Gpmf::new(&path)?;
    ///     Ok(())
    /// }
    /// ```
    // pub fn new(path: &Path) -> Result<Self, GpmfError> {
    pub fn new(path: &Path, debug: bool) -> Result<Self, GpmfError> {
        let ext = path.extension()
            .and_then(|e| e.to_str())
            .map(|s| s.to_lowercase())
            .ok_or_else(|| GpmfError::InvalidFileType(path.to_owned()))?;

        match ext.as_ref() {
            "mp4"
            | "lrv" => Self::from_mp4(path, debug),
            "jpg"
            | "jpeg" => Self::from_jpg(path, debug),
            // Possibly "raw" GPMF-file
            _ => Self::from_raw(path, debug)
        }
    }

    // // Returns the entire GPMF stream unparsed as `Cursor<Vec<u8>>`.
    // pub fn raw(mp4: &mut mp4iter::Mp4) {
        
    // }

    /// Returns first DEVC stream only and without parsing.
    /// 
    /// Presumed to be unique enough to use as a fingerprint
    /// without having to hash the entire GPMF stream or the
    /// MP4 file itself.
    /// 
    /// Used for producing a hash that can be stored in a `GoProFile`
    /// struct to match high and low resolution clips, or duplicate ones.
    pub(crate) fn first_raw(path: &Path) -> Result<Cursor<Vec<u8>>, GpmfError> {
        let mut mp4 = mp4iter::Mp4::new(path)?;
        Self::first_raw_mp4(&mut mp4)
    }

    /// Extracts first DEVC stream only and without parsing,
    /// then creates and return a Blake3 hash of the raw bytes.
    /// 
    /// Presumed to be unqique enough to use as a fingerprint
    /// without having to hash the entire GPMF stream or the
    /// MP4 file itself.
    /// 
    /// Used for producing a hash that can be store in a `GoProFile`
    /// struct to match, or high and low resolution clips or duplicate ones.
    pub(crate) fn first_raw_mp4(mp4: &mut mp4iter::Mp4) -> Result<Cursor<Vec<u8>>, GpmfError> {
        mp4.reset()?;
        let offsets = mp4.offsets("GoPro MET")?;

        if let Some(offset) = offsets.first() {
            mp4.read_at(offset.position, offset.size as u64)
                .map_err(|err| err.into())
        } else {
            Err(GpmfError::NoMp4Offsets)
        }
    }

    /// Returns the embedded GPMF streams in a GoPro MP4 file.
    // pub fn from_mp4(path: &Path) -> Result<Self, GpmfError> {
    pub fn from_mp4(path: &Path, debug: bool) -> Result<Self, GpmfError> {
        let mut mp4 = mp4iter::Mp4::new(path)?;

        // TODO 220812 REGRESSION CHECK: DONE.
        // TODO        Mp4::offsets() 2-3x slower with new code (4GB file),
        // TODO        though in microsecs: 110-200us old vs 240-600us new.
        // 1. Extract position/byte offset, size, and time span for GPMF chunks.
        let offsets = mp4.offsets("GoPro MET")?;
        
        // Faster than a single, serial iter so far.
        // 2. Read data at MP4 offsets and generate timestamps serially
        let mut timestamps: Vec<Timestamp> = Vec::new();
        let mut cursors = offsets.iter()
            .map(|o| {
                // Create timestamp
                let timestamp = timestamps.last()
                    .map(|t| Timestamp {
                        relative: t.relative + o.duration,
                        duration: o.duration,
                    }).unwrap_or(Timestamp {
                        relative: 0,
                        duration: o.duration
                    });
                timestamps.push(timestamp);

                // Read and return data at MP4 offsets
                mp4.read_at(o.position as u64, o.size as u64)
                    .map_err(|e| GpmfError::Mp4Error(e))
            })
            .collect::<Result<Vec<_>, GpmfError>>()?;

        assert_eq!(timestamps.len(), cursors.len(), "Timestamps and cursors differ in number for GPMF");

        // 3. Parse each data chunk/cursor into Vec<Stream>.
        let streams = cursors.par_iter_mut().zip(timestamps.par_iter())
            .map(|(cursor, t)| {
                // let stream = Stream::new(cursor, None)
                let stream = Stream::new(cursor, None, debug)
                    .map(|mut strm| {
                        // 1-2 streams. 1 for e.g. Hero lineup, 2 for Karma drone (1 for drone, 1 for attached cam)
                        strm.iter_mut().for_each(|s| s.set_time(t));
                        strm
                    });
                stream
            })
            .collect::<Result<Vec<_>, GpmfError>>()? // Vec<Vec<Stream>>, need to flatten
            .par_iter()
            .flatten_iter() // flatten will mix drone data with cam data, perhaps bad idea
            .cloned()
            .collect::<Vec<_>>();

        Ok(Self{
            streams,
            source: vec![path.to_owned()]
        })
    }

    /// Returns the embedded GPMF stream in a GoPro photo, JPEG only.
    pub fn from_jpg(path: &Path, debug: bool) -> Result<Self, GpmfError> {
        // Find and extract EXIf chunk with GPMF
        let segment = Jpeg::new(path)?
            .find(&JpegTag::APP6)
            .map_err(|err| GpmfError::JpegError(err))?;

        if let Some(mut app6) = segment {
            app6.seek(6); // seek past `GoPro\null`
            // return Self::from_cursor(&mut app6.data)
            return Self::from_cursor(&mut app6.data, debug)
        } else {
            Err(GpmfError::InvalidFileType(path.to_owned()))
        }
    }

    /// Returns GPMF from a "raw" GPMF-file,
    /// e.g. the "GoPro MET" track extracted from a GoPro MP4 with FFMpeg.
    pub fn from_raw(path: &Path, debug: bool) -> Result<Self, GpmfError> {
        // TODO do a buffered read instead of arbitrary max size value?
        let max_size = 50_000_000_u64; // max in-memory size set to 50MB
        let size = path.metadata()?.len();

        if size > max_size {
            return Err(GpmfError::MaxFileSizeExceeded{
                max: max_size,
                got: size,
                path: path.to_owned()
            })
        }

        let mut cursor = Cursor::new(std::fs::read(path)?);
        // let streams = Stream::new(&mut cursor, None)?;
        let streams = Stream::new(&mut cursor, None, debug)?;

        Ok(Self{
            streams,
            source: vec![path.to_owned()]
        })
    }

    /// GPMF from byte slice.
    // pub fn from_slice(slice: &[u8]) -> Result<Self, GpmfError> {
    pub fn from_slice(slice: &[u8], debug: bool) -> Result<Self, GpmfError> {
        let mut cursor = Cursor::new(slice.to_owned());
        // Self::from_cursor(&mut cursor)
        Self::from_cursor(&mut cursor, debug)
    }

    /// GPMF from `Cursor<Vec<u8>>`.
    // pub fn from_cursor(cursor: &mut Cursor<Vec<u8>>) -> Result<Self, GpmfError> {
    pub fn from_cursor(cursor: &mut Cursor<Vec<u8>>, debug: bool) -> Result<Self, GpmfError> {
        Ok(Self{
            // streams: Stream::new(cursor, None)?,
            streams: Stream::new(cursor, None, debug)?,
            source: vec![]
        })
    }

    pub fn print(&self) {
        self.iter().enumerate()
            .for_each(|(i, s)|
                s.print(Some(i+1), Some(self.len()))
            )
    }

    /// Returns number of `Streams`.
    pub fn len(&self) -> usize {
        self.streams.len()
    }

    pub fn iter(&self) -> impl Iterator<Item = &Stream> {
        self.streams.iter()
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
                return Some(stream)
            }
            match &stream.streams {
                StreamType::Nested(s) => {
                    for strm in s.iter() {
                        strm.find(fourcc);
                    }
                },
                StreamType::Values(_) => return None
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
    pub fn merge_mut(&mut self, gpmf: &mut Self) {
        if let Some(ts) = self.last_timestamp() {
            // adds final timestamp of previous gpmf to all timestamps
            gpmf.offset_time(&ts);
        }

        // Use append() instead?
        // https://github.com/rust-lang/rust-clippy/issues/4321#issuecomment-929110184
        self.extend(&gpmf.streams);
        self.source.extend(gpmf.source.to_owned());
    }

    /// Filters direct child nodes based on `StreamType`. Not recursive.
    pub fn filter(&self, data_type: &DataType) -> Vec<Stream> {
        // self.iter()
        self.streams.par_iter()
            .flat_map(|s| s.filter(data_type))
            .collect()
    }

    /// Filters direct child nodes based on `StreamType` and returns an iterator. Not recursive.
    pub fn filter_iter<'a>(
        &'a self,
        data_type: &'a DataType,
    ) -> impl Iterator<Item = Stream> + 'a {
        // self.iter()
        self.streams.iter()
            .flat_map(move |s| s.filter(data_type))
    }

    /// Returns all unique free text stream descriptions, i.e. `STNM` data.
    /// The hierarchy is `DEVC` -> `STRM` -> `STNM`.
    pub fn types(&self) -> Vec<String> {
        let mut unique: HashSet<String> = HashSet::new();
        for devc in self.streams.iter() {
            devc.find_all(&FourCC::STRM).iter()
                .filter_map(|s| s.name())
                .for_each(|n| _ = unique.insert(n));
        };
        let mut types = unique.into_iter().collect::<Vec<_>>();
        types.sort();
        types
    }

    /// Returns summed duration of MP4 sources (longest track).
    /// Raises error if sources are not MP4-files
    /// (e.g. if source is a raw `.gpmf` extracted via FFmpeg).
    pub fn duration(&self) -> Result<time::Duration, GpmfError> {
        // let dur = self.source.iter()
        //     .fold(time::Duration::ZERO, |acc, path| acc + mp4iter::Mp4::new(path)?.duration()?);
        // Ok(dur)
        let mut duration = time::Duration::ZERO;
        for path in self.source.iter() {
            duration += mp4iter::Mp4::new(path)?.duration()?;
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
            .for_each(|devc|
                devc.time = devc.time.to_owned().map(|t| t.add(time))
            )
    }

    /// Returns first `Timestamp` in GPMF stream.
    pub fn first_timestamp(&self) -> Option<&Timestamp> {
        self.first()
            .and_then(|devc| devc.time.as_ref())
    }
    
    /// Returns last `Timestamp` in GPMF stream.
    pub fn last_timestamp(&self) -> Option<&Timestamp> {
        self.last()
            .and_then(|devc| devc.time.as_ref())
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
        let names_set: HashSet<String> = self.streams.iter()
            .filter_map(|s| s.device_name())
            .collect();
        
        let mut names = Vec::from_iter(names_set);
        names.sort();

        names
    }

    /// Device ID. Extracted from first `Stream`.
    pub fn device_id(&self) -> Option<Dvid> {
        self.streams
            .first()
            .and_then(|s| s.device_id())
    }

    /// For `GPS5` models, Hero10 and earlier.
    /// 
    /// Since `GPS5` only logs datetime, GPS fix, and DOP
    /// per point-cluster, a single average point is returned
    /// per `STRM`. This is a linear average, but
    /// should be accurate enough for the up to 18Hz log.
    /// Implementing a latitude dependent average
    /// is a future possibility.
    pub fn gps5(&self) -> Gps {
        Gps(self.filter_iter(&DataType::Gps5)
            // why is this flat_map and not filter_map?
            .flat_map(|s| GoProPoint::from_gps5(&s)) // TODO which Point to use?
            .collect::<Vec<_>>())
    }

    /// For `GPS9` models, Hero11 and later.
    /// 
    /// Since the newer `GPS9` format logs datetime,
    /// GPS fix, and DOP per-point, all points are returned,
    /// which means larger amounts of data.
    pub fn gps9(&self) -> Gps {
        Gps(self.filter_iter(&DataType::Gps9)
            .filter_map(|s| GoProPoint::from_gps9(&s)) // TODO which Point to use?
            .flatten()
            .collect::<Vec<_>>())
    }

    /// Sensors data. Available sensors depend on model.
    pub fn sensor(&self, sensor_type: &SensorType) -> Vec<SensorData> {
        SensorData::from_gpmf(self, sensor_type)
    }

    // /// Extract custom data in MP4 `udta` container.
    // /// GoPro stores some device settings and info here,
    // /// including a mostly undocumented GPMF-stream.
    // pub fn meta(&self) -> Result<(), GpmfError> {
    //     if self.source.iter().any(|p| !match_extension(p, "mp4")) {
    //         return Err(GpmfError::InvalidFileType{expected_ext: String::from("mp4")})
    //     }
    //     for path in self.source.iter() {
    //         let gpmeta = GoProMeta::new(path)?;
    //     }

    //     Ok(())
    // }

    // /// Derive starting time, i.e. the absolute timestamp for first `DEVC`.
    // /// Can only be determined if the GPS was turned on and logged data.
    // /// 
    // /// Convenience method that simply subtracts first `Point`'s `Point.time.instant` from `Point.datetime`.
    // /// 
    // /// Note that this will filter on Gps streams again,
    // /// so if you already have a `Gps` struct use `Gps::t0()`,
    // /// or do the calucation yourself from `Vec<Point>`.
    // pub fn t0(&self) -> Option<NaiveDateTime> {
    //     self.gps().t0()
    // }
}
