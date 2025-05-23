//! Convenience structure for dealing with relative timestamps.

use mp4iter::Sample;
use time::{self, Duration};

#[derive(Debug, Clone, Default, PartialEq, Eq, PartialOrd)]
/// Timestamp containing relative time in milliseconds from
/// video start and the "duration" (i.e. time until write of next GPMF chunk)
/// of the DEVC the current stream belongs to.
pub struct Timestamp {
    /// Time passed since video start.
    pub relative: Duration,
    /// 'Sample' duration for the `DEVC`,
    /// i.e. time until next `DEVC` is logged.
    pub duration: Duration,
}

impl Ord for Timestamp {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        if self.relative > other.relative {
            return std::cmp::Ordering::Greater
        }
        if self.relative < other.relative {
            return std::cmp::Ordering::Less
        }
        std::cmp::Ordering::Equal
    }
}

impl From<(Duration, Duration)> for Timestamp {
    fn from(value: (Duration, Duration)) -> Self {
        Self {
            relative: value.0,
            duration: value.1,
        }
    }
}

impl From<&Sample> for Timestamp {
    fn from(value: &Sample) -> Self {
        Self {
            relative: value.relative(),
            duration: value.duration(),
        }
    }
}

impl From<&mut Sample> for Timestamp {
    fn from(value: &mut Sample) -> Self {
        Self {
            relative: value.relative(),
            duration: value.duration(),
        }
    }
}

impl Timestamp {
    /// New Timestamp. `relative` equals time in milliseconds
    /// from video start time,
    /// `duration` equals "sample duration" in milliseconds
    /// for the `Stream` it is attached to.
    pub fn new(relative: u32, duration: u32) -> Self {
        Timestamp{
            relative: Duration::milliseconds(relative as i64),
            duration: Duration::milliseconds(duration as i64),
        }
    }

    /// Returns `Timestamp.relative` (relative to video start)
    /// as milliseconds.
    pub fn relative_ms(&self) -> i128 {
        self.relative.whole_milliseconds()
    }

    /// Returns `Timestamp.duration` (duration of current DEVC chunk)
    /// as `time::Duration`.
    pub fn duration_ms(&self) -> i128 {
        self.duration.whole_milliseconds()
    }

    /// Adds one stream `Timestamp` to another
    /// and returns the resulting `Timestamp`.
    /// Only modifies the `relative` field.
    ///
    /// Order is unfortunately critical: `other`'s value is used to
    /// extend `self`, not the other way around.
    /// This is due to `other`'s sample duration (derived from MP4 timing
    /// via the `stts` atom) being involved.
    /// For other MP4 tracks sample durations
    /// may vary throughout the track. This is so far not the case
    /// for the GPMF track (`GoPro MET`).
    pub fn add(&self, other: &Self) -> Self {
        Self {
            // relative: self.relative + other.relative,
            relative: self.relative + other.relative + other.duration, // need duration as well
            ..self.to_owned()
        }
    }

    // Removed subtraction since it's not clear in what situation this is needed or how it should be implemented
    // /// Substracts one `Timestamp` from another and returns the resulting `Timestamp`.
    // /// Only modifies the `relative` field.
    // pub fn sub(&self, timestamp: &Self) -> Self {
    //     Self {
    //         // relative: self.relative - timestamp.relative,
    //         relative: self.relative - timestamp.relative - timestamp.duration, // doesnt make sense
    //         ..self.to_owned()
    //     }
    // }
}
