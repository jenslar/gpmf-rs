# v0.5
- NEW: Initial Hero 13 Black compatibility. Only verified against a single, short sample.
- BREAKING: `GoProSession::sessions_from_path()` now silently continues if track `GoPro MET` is not found (previous version raised error).
- BREAKING: `Gps::prune()` accept optional GPS satellite lock level (`0`, `2` = 2D lock, `3` = 3D), and optional dilution of precision (below 5.0 is usually good).

# v0.4
- NOTE: Hero 13 Black compatibility is unknown until I get hold of sample files (it once again has a GPS module)
- BREAKING: Methods for locating/grouping files in recording session now return `Result` with optional "skip on error".
- NEW: determining high/low resolution clip no longer depends on file-extension, only resolution, where 1920 x 1080 is used as the minimum for determining whether a clip is high-resolution (`.MP4`) or low-resolution (`.LRV`).
- FIX: Fixed overlapping timestamps when merging GPMF data from multiple MP4-files.

# v0.3.1
- Internal changes

# v0.2.0
- Fixed export of coordinates for `GPS9` devices (Hero11 and later)