//! GoPro recording session.

use std::{
    path::{Path, PathBuf},
    collections::HashMap
};

use time::{PrimitiveDateTime, Duration};
use walkdir::WalkDir;

use crate::{
    Gpmf,
    GpmfError,
    DeviceName,
    files::has_extension,
    Gps
};

use super::{GoProFile, GoProMeta};

#[derive(Debug, Default, Clone, PartialEq)]
pub struct GoProSession(Vec<GoProFile>);

impl GoProSession {
    pub fn len(&self) -> usize {
        self.0.len()
    }

    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    pub fn add(&mut self, gopro_file: &GoProFile) {
        self.0.push(gopro_file.to_owned());
    }

    pub fn append(&mut self, gopro_files: &[GoProFile]) {
        self.0.append(&mut gopro_files.to_owned());
    }

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

    pub fn device(&self) -> Option<&DeviceName> {
        self.first().map(|gp| &gp.device)
    }

    /// Parses and merges GPMF-data for all
    /// files in session to a single `Gpmf` struct.
    pub fn gpmf(&self) -> Result<Gpmf, GpmfError> {
        let mut gpmf = Gpmf::default();
        for gopro in self.iter() {
            gpmf.merge_mut(&mut gopro.gpmf()?);
        }
        Ok(gpmf)
    }

    /// Extracts custom user data in MP4 `udta`
    /// atom for all clips. GoPro models later than
    /// Hero5 Black embed an undocumented
    /// GPMF stream here that is also included.
    pub fn meta(&self) -> Vec<GoProMeta> {
        self.0.iter()
            .filter_map(|gp| gp.meta().ok())
            .collect()
    }

    /// Returns paths to high-resolution MP4-clips if set (`.MP4`).
    pub fn mp4(&self) -> Vec<PathBuf> {
        self.iter()
            .filter_map(|f| f.mp4.to_owned())
            .collect()
    }

    /// Returns paths to low-resolution MP4-clips if set (`.LRV`).
    pub fn lrv(&self) -> Vec<PathBuf> {
        self.iter()
            .filter_map(|f| f.lrv.to_owned())
            .collect()
    }

    /// Returns `true` if paths are set for all high-resolution clips in session.
    pub fn matched_lo(&self) -> bool {
        match self.iter().any(|gp| gp.lrv.is_none()) {
            true => false,
            false => true
        }
    }

    /// Returns `true` if paths are set for all low-resolution clip in session.
    pub fn matched_hi(&self) -> bool {
        match self.iter().any(|gp| gp.mp4.is_none()) {
            true => false,
            false => true
        }
    }

    /// Sort GoPro clips in session based on GPS timestamps (meaning
    /// GPS is required). No other continuous timeline for the session exists.
    /// 
    /// To speed things up only the first DEVC container is used for sorting.
    /// It should be sufficient since even if the device has not yet acquired
    /// a GPS lock, the timestamp should still precede those in the clips
    /// that follow.
    /// 
    /// `prune = true` prunes files that return an error on parsing the GPMF stream.
    /// Will otherwise fail on first corrupt file.
    pub fn sort_gps(&mut self, prune: bool) -> Result<Self, GpmfError> {
        let mut dt_gp: Vec<(PrimitiveDateTime, GoProFile)> = Vec::new();
        let mut remove_index: Vec<usize> = Vec::new();

        for (i, gp) in self.0.iter_mut().enumerate() {
            // Extract GPS log, and filter out points with bad satellite lock
            let mut gps = Gps::default();
            if prune {
                // TODO testing using only the first DEVC, since timestamp
                // TODO while incorrect should still be in chronological order
                if let Ok(gpmf) = gp.gpmf_first() {
                    gps = gpmf.gps().prune(2, None); // now checks model, uses gps9 for hero11 gps5 otherwise
                } else {
                    // Add index of GoProFile for removal for file
                    // that raised error then continue to next file
                    remove_index.push(i);
                    continue;
                }
            } else {
                gps = gp.gpmf_first()?.gps().prune(2, None); // now checks model, uses gps9 for hero11 gps5 otherwise
            }
            // Return error if no points were logged
            // If one file in sequence does not contains GPS data,
            // neither should any of the other since GoPro logs
            // last known location with no satellite lock.
            // I.e. no points at all indicates the GPS was turned off.
            if gps.len() == 0 {
                return Err(GpmfError::NoData)
            }
            if let Some(t) = gps.first().map(|p| p.datetime) {
                dt_gp.push((t, gp.to_owned()))
            }
        }

        // Pruning added indeces for files that raised errors, if prune = true
        for index in remove_index.iter() {
            self.remove(*index)
        }

        // Sort remaining files by first good GPS datetime in each file.
        dt_gp.sort_by_key(|(t, _)| t.to_owned());

        Ok(Self(dt_gp.iter().map(|(_, gp)| gp.to_owned()).collect::<Vec<_>>()))
    }

    /// Sort GoPro clips in session based on filename.
    /// Presumes clips are named to represent chronological order
    /// (GoPro's own file naming convention works).
    pub fn sort_filename(&self) -> Result<Self, GpmfError> {
        // Ensure all paths are set for at least one resolution
        let sort_on_hi = match (self.matched_hi(), self.matched_lo()) {
            (true, _) => true,
            (false, true) => false,
            (false, false) => return Err(GpmfError::PathNotSet)
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
    pub fn from_path(video: &Path, dir: Option<&Path>, verbose: bool) -> Option<Self> {
        let indir = match dir {
            Some(d) => d,
            None => video.parent()?
        };
        let sessions = Self::sessions_from_path(indir, Some(video), verbose);
        
        sessions.first().cloned()
    }

    /// Locate and group clips belonging to the same
    /// recording session. Only returns unique files: if the same
    /// file is encounterd twice it will only yield a single result.
    /// I.e. this function is not intended to be a "find all GoPro files",
    /// only "find and group unique GoPro files".
    pub fn sessions_from_path(
        dir: &Path,
        video: Option<&Path>,
        verbose: bool
    ) -> Vec<Self> {
        // Key = Blake3 hash as Vec<u8> of extracted GPMF raw bytes 
        let mut hash2gopro: HashMap<Vec<u8>, GoProFile> = HashMap::new();

        let gopro_in_session = match video {
            Some(p) => {
                GoProFile::new(p).ok()
            },
            _ => None
        };

        let mut count = 0;

        // 1. Go through files, set 
        for result in WalkDir::new(dir) {
            let path = match result {
                Ok(f) => f.path().to_owned(),
                // Ignore errors, since these are often due to lack of read permissions
                Err(_) => continue
            };

            // Currently only know how mp4+lrv matches for hero11,
            // and how mp4 (not lrv) matches for hero7
            // As for setting both MP4 and LRV path,
            // `GoProFile::new()` checks parent folder only
            // The above may mean the same file may be
            // processed twice.
            // if has_extension(&path, "mp4") | has_extension(&path, "lrv") {
            if let Some(ext) = has_extension(&path, &["mp4", "lrv"]) {
                if let Ok(gp) = GoProFile::new(&path) {
                    if verbose {
                        count += 1;
                        println!("{:4}. [{:12} {}] {}",
                            count,
                            gp.device.to_str(),
                            ext.to_uppercase(),
                            path.display());
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
                            },
                            _ => {
                                if gp.gumi != gp_session.gumi {
                                    continue;
                                }
                            }
                        }
                    }

                    // `set_path()` sets MP4 or LRV path based on file extension
                    hash2gopro.entry(gp.fingerprint.to_owned())
                        .or_insert(gp).set_path(&path);
                }
            }
        }

        // 2. Group files on MUID or GUMI depending on model

        // Group clips with the same full MUID ([u32; 8])
        let mut muid2gopro: HashMap<Vec<u32>, Vec<GoProFile>> = HashMap::new();
        // Group clips with the same full GUMI ([u8; 16])
        let mut gumi2gopro: HashMap<Vec<u8>, Vec<GoProFile>> = HashMap::new();
        for (_, gp) in hash2gopro.iter() {
            match gp.device {
                // Hero 11 uses the same MUID for clips in the same session.
                DeviceName::Hero11Black => muid2gopro
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

        if verbose {
            println!("Compiling and sorting sessions...")
        }

        // Compile all sessions
        let mut sessions = muid2gopro.iter()
            .map(|(_, sessions1)| GoProSession(sessions1.to_owned()))
            .chain(
                gumi2gopro.iter()
                .map(|(_, sessions2)| GoProSession(sessions2.to_owned()))
            )
            .collect::<Vec<_>>();

        // 3. Sort files within groups on GPS datetime to determine sequence
        // TODO possible that duplicate files (with different paths) will be included
        let sorted_sessions = sessions.iter_mut()
            .filter_map(|s| if s.len() == 1 {
                // Avoid parsing GPS to sort for single-clip sessions
                Some(s.to_owned())
            } else {
                s.sort_gps(true).ok()
            })
            .collect::<Vec<_>>();

        sorted_sessions
    }

    /// Returns duration of session.
    pub fn duration(&self) -> Result<Duration, GpmfError> {
        self.iter()
            .map(|g| g.duration())
            .sum()
    }

    /// Returns duration of session as milliseconds.
    pub fn duration_ms(&self) -> Result<i64, GpmfError> {
        self.duration()?
            .whole_milliseconds()
            .try_into()
            .map_err(|err| GpmfError::DowncastIntError(err))
    }
}