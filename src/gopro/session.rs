//! GoPro recording session. Container for `GoProFile`, listing all clips that belong
//! to one recording session chronologically.

use std::{
    collections::{HashMap, HashSet}, hash::{DefaultHasher, Hash, Hasher}, path::{Path, PathBuf}
};

use indicatif::{ParallelProgressIterator, ProgressBar};
use mp4iter::Mp4Error;
use rayon::iter::{IntoParallelIterator, IntoParallelRefIterator, ParallelIterator};
use time::{Duration, PrimitiveDateTime};
use walkdir::WalkDir;

use crate::{files::has_extension, DeviceName, Gpmf, GpmfError};

use super::{GoProFile, GoProMeta};

#[derive(Debug, Default, Clone, PartialEq)]
pub struct GoProSession(Vec<GoProFile>);

impl Hash for GoProSession {
    /// A combined hash of the fingerprints for each, respective
    /// `GoProFile` in this session.
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.0.iter().map(|f| f.fingerprint.to_owned())
            .collect::<Vec<_>>()
            .hash(state);
    }
}

impl GoProSession {
    /// Number of clips in session.
    pub fn len(&self) -> usize {
        self.0.len()
    }

    /// Returns `true` if session contains no clips.
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    /// Returns a fingerprint/ID for the `GoProSession`,
    /// consiting of hashed fingerprint of the individual
    /// `GoProFile`s.
    pub fn fingerprint(&self) -> u64 {
        let mut hasher = DefaultHasher::new();
        self.hash(&mut hasher);
        hasher.finish()
    }

    /// Add `GoProFile` last to session.
    pub fn add(&mut self, gopro_file: &GoProFile) {
        self.0.push(gopro_file.to_owned());
    }

    pub fn append(&mut self, gopro_files: &[GoProFile]) {
        self.0.append(&mut gopro_files.to_owned());
    }

    /// Remove file via its vector index.
    pub fn remove(&mut self, index: usize) {
        self.0.remove(index);
    }

    pub fn iter(&self) -> impl Iterator<Item = &GoProFile> {
        self.0.iter()
    }

    pub fn iter_mut(&mut self) -> impl Iterator<Item = &mut GoProFile> {
        self.0.iter_mut()
    }

    pub fn first(&self) -> Option<&GoProFile> {
        self.0.first()
    }

    pub fn first_mut(&mut self) -> Option<&mut GoProFile> {
        self.0.first_mut()
    }

    pub fn last(&self) -> Option<&GoProFile> {
        self.0.last()
    }

    pub fn last_mut(&mut self) -> Option<&mut GoProFile> {
        self.0.last_mut()
    }

    pub fn as_slice(&self) -> &[GoProFile] {
        &self.0
    }

    /// Derive 'basename' for session from first clip in session,
    /// high-res or low-res clip (prioritized).
    /// E.g. if session contains `GH010026.MP4, GH020026.MP4, GH030026.MP4`,
    /// `GH010026` will be returned.
    pub fn basename(&self) -> Option<String> {
        let path = self
            .first()
            .and_then(|gp| gp.mp4.as_deref().or_else(|| gp.lrv.as_deref()));

        if let Some(p) = path {
            p.file_stem().map(|s| s.to_string_lossy().to_string())
        } else {
            None
        }
    }

    /// Returns device name for camera used.
    pub fn device(&self) -> Option<&DeviceName> {
        self.first().map(|gp| &gp.device)
    }

    /// Returns device serial number for camera used.
    /// (extracted from `CAME` in `udta` atom).
    /// Panics if more than one unique serial is found.
    pub fn serial(&self) -> Vec<u8> {
        let serials: HashSet<Vec<u8>> = self.iter()
                .filter_map(|gp| {
                    gp.serial().ok()
                })
                .collect();

        assert!(serials.len() == 1, "Found multiple camera serial numbers in single session");

        serials.iter().nth(0).unwrap().to_owned()
    }

    /// Create a session from a single clip.
    pub fn single(path: &Path) -> Result<Self, GpmfError> {
        Ok(Self(vec![GoProFile::new(path)?]))
    }

    /// Parses and merges GPMF-data for all
    /// files in session to a single `Gpmf` struct.
    pub fn gpmf(&self) -> Result<Gpmf, GpmfError> {
        let mut gpmf = Gpmf::default();
        for file in self.iter() {
            gpmf.merge_mut(&mut file.gpmf()?);
        }
        Ok(gpmf)
    }

    /// Extracts custom user data in MP4 `udta`
    /// atom for all clips. GoPro models later than
    /// Hero5 Black embed an undocumented
    /// GPMF stream here that is also included.
    pub fn meta(&self) -> Vec<GoProMeta> {
        self.0.iter().filter_map(|gp| gp.meta().ok()).collect()
    }

    /// Returs all paths to either high-resolution clips,
    /// or low-resolution clips in session, whichever is set,
    /// priotritising high-resolution, skipping
    /// `GoProFile`s that have no path set.
    pub fn paths(&self) -> Vec<PathBuf> {
        self
            .iter()
            .filter_map(|f| f.path().ok())
            .map(|p| p.to_owned())
            .collect()
    }

    /// Returns paths to high-resolution MP4-clips if set (`.MP4`),
    /// skipping `GoProFile`s that have no path set.
    pub fn mp4(&self) -> Vec<PathBuf> {
        self
            .iter()
            .filter_map(|f| f.mp4.to_owned())
            .collect()
    }

    /// Returns paths to low-resolution MP4-clips if set (`.LRV`),
    /// skipping `GoProFile`s that have no path set.
    pub fn lrv(&self) -> Vec<PathBuf> {
        self
            .iter()
            .filter_map(|f| f.lrv.to_owned())
            .collect()
    }

    /// Returns `true` if paths are set for all high-resolution clips in session.
    pub fn matched_lo(&self) -> bool {
        !self.iter().any(|gp| gp.lrv.is_none())
    }

    /// Returns `true` if paths are set for all low-resolution clip in session.
    pub fn matched_hi(&self) -> bool {
        !self.iter().any(|gp| gp.mp4.is_none())
    }

    pub fn offsets(&self) {
        // let mp4 = self.0
    }

    /// Sort clips chronologically by `GoProFile::time_first_frame`.
    ///
    /// This is so far the only timestamp that is
    /// progressive across clips in the same session.
    /// MP4 creation time in `mvhd` atom will have the same
    /// date logged for all GoPro clips belonging to the same
    /// recordig session.
    pub fn sort(&mut self) {
        self.0.sort_by_key(|k| k.time_first_frame)
        // sorts by MP4 creation time which will be the same for some gopro devices
        // self.0.sort_by_key(|k| k.start())
    }

    /// Sort GoPro clips in session based on filename.
    /// Presumes clips are named to represent chronological order
    /// (GoPro's own file naming convention works).
    pub fn sort_filename(&self) -> Result<Self, GpmfError> {
        // Ensure all paths are set for at least one resolution
        let sort_on_hi = match (self.matched_hi(), self.matched_lo()) {
            (true, _) => true,
            (false, true) => false,
            (false, false) => return Err(GpmfError::PathNotSet),
        };
        let mut files = self.0.to_owned();
        files.sort_by_key(|gp| {
            if sort_on_hi {
                gp.mp4.to_owned().unwrap() // checked that path is set above
            } else {
                gp.lrv.to_owned().unwrap() // checked that path is set above
            }
        });

        Ok(Self(files))
    }

    /// Find all clips in session containing `video`.
    /// `dir` is the starting point for searching for clips.
    /// If `dir` is `None` the parent dir of `video` is used.
    pub fn from_path(
        video: &Path,
        dir: Option<&Path>,
        verify_gpmf: bool,
        verbose: bool,
        continue_on_error: bool
    ) -> Result<Self, GpmfError> {
        let indir = match dir {
            Some(d) => d,
            None => video.parent().ok_or(GpmfError::NoParentDir)?,
        };

        Self::sessions_from_path(indir, Some(video), verify_gpmf, verbose, continue_on_error)?
            .first()
            .cloned()
            .ok_or(GpmfError::NoSession)
    }

    /// Determines recording session from `GoProFile`. If `dir` is `None`,
    /// the parent dir of clips/s in `GoProFile` will be used.
    pub fn from_goprofile(
        gopro: &GoProFile,
        dir: Option<&Path>,
        verify_gpmf: bool,
        continue_on_error: bool
    ) -> Result<Self, GpmfError> {
        let indir = match dir {
            Some(d) => d,
            None => gopro.path()?.parent().ok_or(GpmfError::NoParentDir)?,
        };

        Self::sessions_from_path(indir, Some(gopro.path()?), verify_gpmf, false, continue_on_error)?
            .first()
            .cloned()
            .ok_or(GpmfError::NoSession)
    }

    /// Locate and group clips belonging to the same
    /// recording session. Only returns unique files: if the same
    /// file is encounterd twice it will only yield a single result.
    /// I.e. this function is not intended to be a "find all GoPro files",
    /// only "find and group unique GoPro files".
    ///
    /// `verify_gpmf` does a full parse on each GoPro file, and discards
    /// corrupt ones.
    pub fn sessions_from_path(
        dir: &Path,
        video: Option<&Path>,
        verify_gpmf: bool,
        verbose: bool,
        continue_on_error: bool,
    ) -> Result<Vec<Self>, GpmfError> {
        // Key = Blake3 hash as Vec<u8> of extracted GPMF raw bytes
        // TODO below should be Vec<GoProFile> then use first one that produces GPMF with no errors when sorting
        let mut hash2gopro: HashMap<Vec<u8>, GoProFile> = HashMap::new();

        let gopro_in_session = match video {
            Some(p) => GoProFile::new(p).ok(),
            _ => None,
        };

        let mut count = 0;

        // 1. Go through files, set
        for result in WalkDir::new(dir) {
            let path = match result {
                Ok(f) => f.path().to_owned(),
                // Ignore errors, since these are often due to lack of read permissions
                Err(_) => continue,
            };

            if let Some(ext) = has_extension(&path, &["mp4", "lrv"]) {
                let gp_result = GoProFile::new(&path);
                let gp = match gp_result {
                    Ok(gp) => gp,
                    Err(err) => if continue_on_error {
                        continue;
                    } else {
                        match err {
                            // Always continue on error due to no "GoPro MET" track
                            GpmfError::Mp4Error(Mp4Error::NoSuchTrack(_)) => {
                                continue;
                            },
                            _ => return Err(err)
                        }
                    },
                };
                if verbose {
                    count += 1;
                    print!(
                        "{:4}. [{:12} {}] {}",
                        count,
                        gp.device.to_str(),
                        ext.to_uppercase(),
                        path.display()
                    );
                }

                // Optionally do a full GPMF parse to prune
                // corrupt files (will otherwise possibly overwrite entry in hashmap)
                if verify_gpmf {
                    if let Err(_err) = gp.gpmf() {
                        println!(" [SKIPPING: GPMF ERROR]");
                        continue;
                    } else {
                        println!(" [GPMF OK]");
                    }
                } else {
                    println!("");
                }

                if let Some(gp_session) = &gopro_in_session {
                    if gp.device != gp_session.device {
                        continue;
                    }
                    match gp.device {
                        DeviceName::Hero11Black => {
                            if gp.muid != gp_session.muid {
                                continue;
                            }
                        }
                        _ => {
                            if gp.gumi != gp_session.gumi {
                                continue;
                            }
                        }
                    }
                }

                // `set_path()` sets MP4 or LRV path based on file extension
                hash2gopro
                    .entry(gp.fingerprint.to_owned())
                    .or_insert(gp.clone())
                    .merge(&gp)?;
                    // .set_path(&path);
                // }
            }
        }

        // 2. Group files on MUID or GUMI depending on model
        if verbose {
            println!("Compiling and sorting sessions...")
        }

        // Group clips with the same full MUID ([u32; 8])
        // let mut muid2gopro: HashMap<Vec<u32>, Vec<GoProFile>> = HashMap::new();
        let mut muid2gopro: HashMap<[u32; 8], Vec<GoProFile>> = HashMap::new();
        // Group clips with the same full GUMI ([u8; 16]) reading as [u32; 4]
        // let mut gumi2gopro: HashMap<Vec<u8>, Vec<GoProFile>> = HashMap::new();
        let mut gumi2gopro: HashMap<[u32; 4], Vec<GoProFile>> = HashMap::new();
        for (_, gp) in hash2gopro.iter() {
            match gp.device {
                // Hero 11 uses the same MUID for clips in the same session.
                // Currently an assumption that so do Hero 12 and Hero 13.
                DeviceName::Hero11Black
                | DeviceName::Hero12Black
                | DeviceName::Hero13Black => muid2gopro
                    .entry(gp.muid.to_owned())
                    .or_insert(Vec::new())
                    .push(gp.to_owned()),
                // Hero7 uses GUMI. Others unknown, GUMI is a pure guess.
                _ => gumi2gopro
                    .entry(gp.gumi.to_owned())
                    .or_insert(Vec::new())
                    .push(gp.to_owned()),
            };
        }

        // Compile all sessions
        let mut sessions = muid2gopro
            .iter()
            .map(|(_, session)| Self(session.to_owned()))
            .chain(
                gumi2gopro
                    .iter()
                    .map(|(_, session)| Self(session.to_owned())),
            )
            .collect::<Vec<_>>();

        // 3. Sort files within groups on time of first frame since midnight
        // FIXED? TODO possible that duplicate files (with different paths) will be included
        sessions.iter_mut()
            .for_each(|s| s.sort());

        Ok(sessions)
    }

    pub fn sessions_from_path_par(
        dir: &Path,
        video: Option<&Path>,
        verify_gpmf: bool,
        verbose: bool,
        inspect_format: Option<fn(&Path, Option<usize>) -> String>,
    ) -> Vec<Self> {
        // Key = Blake3 hash as Vec<u8> of extracted GPMF raw bytes
        // TODO below should be Vec<GoProFile> then use first one that produces GPMF with no errors when sorting
        // let mut hash2gopro: HashMap<Vec<u8>, GoProFile> = HashMap::new();

        let gopro_in_session = video.and_then(|p| GoProFile::new(p).ok());

        // let mut count = 0;

        println!("Compiling paths...");
        let paths = paths(dir, &["mp4", "lrv"], inspect_format);
        println!("Done ({} candidates found)", paths.len());
        println!("Compiling GoPro files...");
        let files = compile(&paths, verify_gpmf);
        println!("Done ({} GoPro files verified)", files.len());
        println!("Compiling GoPro sessions...");
        let hash2gopro = hash2gopro(&files);
        println!("Done ({} GoPro sessions found)", hash2gopro.len());

        // 2. Group files on MUID or GUMI depending on model

        // Group clips with the same full MUID ([u32; 8])
        // let mut muid2gopro: HashMap<Vec<u32>, Vec<GoProFile>> = HashMap::new();
        let mut muid2gopro: HashMap<[u32; 8], Vec<GoProFile>> = HashMap::new();
        // Group clips with the same full GUMI ([u8; 16]) reading as [u32; 4]
        // let mut gumi2gopro: HashMap<Vec<u8>, Vec<GoProFile>> = HashMap::new();
        let mut gumi2gopro: HashMap<[u32; 4], Vec<GoProFile>> = HashMap::new();
        for (_, gp) in hash2gopro.iter() {
            match gp.device {
                // Hero 11 uses the same MUID for clips in the same session.
                // Currently an assumption that so do Hero 12 and Hero 13.
                DeviceName::Hero11Black
                | DeviceName::Hero12Black
                | DeviceName::Hero13Black => muid2gopro
                    .entry(gp.muid.to_owned())
                    .or_insert(Vec::new())
                    .push(gp.to_owned()),
                // Hero7 uses GUMI. Others unknown, GUMI is a pure guess.
                _ => gumi2gopro
                    .entry(gp.gumi.to_owned())
                    .or_insert(Vec::new())
                    .push(gp.to_owned()),
                // // Hero 11 uses the same MUID for clips in the same session.
                // DeviceName::Hero11Black => muid2gopro
                //     .entry(gp.muid.to_owned())
                //     .or_insert(Vec::new())
                //     .push(gp.to_owned()),
                // // Hero7 uses GUMI. Others unknown, GUMI is a pure guess.
                // _ => gumi2gopro
                //     .entry(gp.gumi.to_owned())
                //     .or_insert(Vec::new())
                //     .push(gp.to_owned()),
            };
        }

        // println!("MUID {muid2gopro:#?}");
        // println!("GUMI {gumi2gopro:#?}");

        if verbose {
            println!("Compiling and sorting sessions...")
        }

        // Compile all sessions
        let mut sessions = muid2gopro
            .iter()
            .map(|(_, session)| Self(session.to_owned()))
            .chain(
                gumi2gopro
                    .iter()
                    .map(|(_, session)| Self(session.to_owned())),
            )
            .collect::<Vec<_>>();

        // 3. Sort files within groups on time of first frame since midnight
        // TODO possible that duplicate files (with different paths) will be included
        sessions.iter_mut()
            .for_each(|s| s.sort());

        if let Some(gp) = gopro_in_session {
            sessions.iter()
                .find_map(|s| if s.part_of(&gp) {Some(vec![s.to_owned()])} else {None})
                .unwrap_or(Vec::new())
        } else {
            sessions
        }
    }

    pub fn start(&self) -> Option<PrimitiveDateTime> {
        self.first()
            // .map(|f| f.creation_time)
            .map(|f| f.start())
    }

    pub fn end(&self) -> Option<PrimitiveDateTime> {
        // Some(self.start()? + self.duration().ok()?)
        Some(self.start()? + self.duration())
    }

    /// Returns duration of session.
    // pub fn duration(&self) -> Result<Duration, GpmfError> {
    pub fn duration(&self) -> Duration {
        self.iter().map(|g| g.duration()).sum()
    }

    /// Returns duration of session as milliseconds.
    // pub fn duration_ms(&self) -> Result<i64, GpmfError> {
    pub fn duration_ms(&self) -> i128 {
        // self.duration()?
        self.duration()
            .whole_milliseconds()
            // .try_into()
            // .map_err(|err| GpmfError::DowncastIntError(err))
    }

    pub fn part_of(&self, gopro: &GoProFile) -> bool {
        self.iter().any(|gp| gopro.matches(gp))
    }

    // combine goprofile fingerprints to generate unique id for session.
    // pub fn fingerprint()
}

fn paths(dir: &Path, ext: &[&str], inspect_format: Option<fn(&Path, Option<usize>) -> String>) -> Vec<PathBuf> {
    WalkDir::new(dir)
        .into_iter()
        .filter_map(|result| if let Ok(entry) = result {
            let p = entry.path();
            // let e = p.extension().and_then(|e| e.to_ascii_lowercase().to_str());
            if let Some(e) = p.extension().map(|e| e.to_string_lossy().to_ascii_lowercase()) {
                if ext.contains(&e.as_str()) {
                    Some(p.to_owned())
                } else {
                    None
                }
            } else {
                None
            }
        } else {
            None
        })
        .enumerate()
        .inspect(|(i, p)| if let Some (f) = inspect_format {
            println!("{}", f(p, Some(*i)));
        })
        .map(|(_, p)| p)
        .collect()
}

// fn hash2gopro(paths: &[PathBuf], verify_gpmf: bool) -> HashMap<Vec<u8>, GoProFile> {
// fn compile(paths: &[PathBuf]) -> HashMap<Vec<u8>, GoProFile> {
// fn compile(paths: &[PathBuf], verify_gpmf: bool, inspect_format: Option<fn(&Path, usize) -> String>) -> Vec<(GoProFile, PathBuf)> {
fn compile(paths: &[PathBuf], verify_gpmf: bool) -> Vec<(GoProFile, PathBuf)> {
    // paths.par_iter()
    //     .map(|path| GoProFile::new(path))
    //     .collect::<Result<Vec<GoProFile>, GpmfError>>()
    let progress = ProgressBar::new(paths.len() as u64);
    paths.par_iter()
        // .progress()
        // Passing on input path as well to ensure it's used in the next step
        .filter_map(|path| {
            let gp = GoProFile::new(&path).ok()?;
            // dbg!(&gp);
            match verify_gpmf {
                true => if gp.gpmf().is_ok() {
                    // println!("[{:12}] {} [OK]", gp.device.to_str(), path.display());
                    Some((gp, path.to_owned()))
                } else {
                    // println!("[{:12}] {} [ERROR: SKIPPING]", gp.device.to_str(), path.display());
                    None
                },
                false => Some((gp, path.to_owned()))
            }
        })
        .inspect(|_| progress.inc(1))
        .collect()
}

fn hash2gopro(files: &[(GoProFile, PathBuf)]) -> HashMap<Vec<u8>, GoProFile> {
    let mut hash2gopro: HashMap<Vec<u8>, GoProFile> = HashMap::new();
    for (gp, path) in files.iter() {
        hash2gopro
            .entry(gp.fingerprint.to_owned())
            .or_insert(gp.to_owned())
            .set_path(&path);
    }
    hash2gopro
}
