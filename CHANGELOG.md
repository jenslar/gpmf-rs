# v0.6.2
- NEW: Added `Gps::downsample()` and `Gps::downsample_mut()` methods.

# v0.6.1
- NEW: Added GPX `From` implementation for `Gps` and `Gpmf`, and GPX `TryFrom` implementation for `GoProFile`, `GoProSession` and `GoProMultiSession`. If possible use the `Gps` implementation. This allows for pruning bad points before export (`Gps::prune()`), since GoPro cameras (at least tested models) usually log dummy points before a satellite lock has been acquired - sometimes a cached, last known location.

# v0.6.0
- BREAKING: `GoProSession` no longer lists both high (`.MP4`) and low-resolution (`.LRV`) clips. Instead, it will match the resolution of the input clip and ignore other resolutions.
- NEW: `GoProSession::locate` is the new method for deriving which clips belong to the same recording session. Only clips with the same resolution as the input clip will be considered.
- NEW: `GoProMultiSession::locate` is the new type and method to locate clips in both resolutions that belong to the same recording session.
- NEW: Added `has_gps() -> bool` method for `GoProFile`, `GoProSession`, and  `GoProMultiSession`. Note that this reads from disk.
- NEW: Added more models, including Hero 13 Black and Mission 1 (note that only Hero5 Black and later have GPMF support).
- NEW: Simple GPX export/write (enable feature "gpx", and use `Gps::to_gpx()` or `Gps::write_gpx()`). Note that this is currently using a [specific commit of the github version](https://github.com/georust/gpx/tree/7ad83e33e64350c6a4893243b77bb3e474db56b5) of the GPX crate since there was no obvious way to create a a new/default `Gpx` object in version 0.10.

# v0.5.3
- FIX: Correctly identifies Hero 10 Black clips in the same recording session (`MUID`/`GUID` check). Temporarily added Hero 12 + 13 to use the same clip identification method as Hero 10 + Hero 11, but this is untested.
- NEW: Added convenience methods `GoProSession::gps()`, `GoProSession::sensor()`. Note that these will read directly from disk each time. If you need multiple data types it may be faster to instead run `GoProSession::gpmf()` which creates in-memory GPMF data, then use the appropriate extraction methods on that.
- BREAKING: Changed the fields `GoProFile::mp4` (high-res video) and `GoProFile::lrv` (low-res video) to the more descriptive `GoProFile::video_high` and `GoProFile::video_low`.

# v0.5.2
- FIX (or BREAKING): `GoProSession::sessions_from_path()` now silently skips any MP4/LRV-file if file name starts with `._`. On macOS these so far contain only quarantine attributes and are regardless not valid MP4-files.
- Bumped crate versions.

# v0.5.1
- Bumped crate versions.

# v0.5
- NEW: Initial Hero 13 Black compatibility. Only verified against a single, short sample.
- BREAKING: `GoProSession::sessions_from_path()` now silently continues if track `GoPro MET` is not found (previous version raised error).
- BREAKING: `Gps::prune()` accept optional GPS satellite lock level (`0` = no lock, `2` = 2D lock, `3` = 3D), and optional dilution of precision (below 5.0 is usually good).

# v0.4
- NOTE: Hero 13 Black compatibility is unknown until I get hold of sample files (it once again has a GPS module)
- BREAKING: Methods for locating/grouping files in recording session now return `Result` with optional "skip on error".
- NEW: determining high/low resolution clip no longer depends on file-extension, only resolution, where 1920 x 1080 is used as the minimum for determining whether a clip is high-resolution (`.MP4`) or low-resolution (`.LRV`).
- FIX: Fixed overlapping timestamps when merging GPMF data from multiple MP4-files.

# v0.3.1
- Internal changes

# v0.2.0
- Fixed export of coordinates for `GPS9` devices (Hero11 and later)
