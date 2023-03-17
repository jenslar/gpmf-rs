//! Convenience structure for dealing with relative timestamps.

use time;

#[derive(Debug, Clone, Default, PartialEq, Eq, PartialOrd)]
/// Timestamp containing relative time in milliseconds from
/// video start and the "duration" (i.e. time until write of next GPMF chunk)
/// of the DEVC the current stream belongs to. 
pub struct Timestamp {
    /// Duration in milliseconds from video start.
    pub relative: u32,
    /// Duration in milliseconds for the `DEVC`
    /// the current stream belongs to.
    pub duration: u32,
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

impl Timestamp {
    /// New Timestamp. `relative` equals time in milliseconds
    /// from video start time,
    /// `duration` equals "sample duration" in milliseconds
    /// for the `Stream` it is attached to.
    pub fn new(relative: u32, duration: u32) -> Self {
        Timestamp{
            relative,
            duration
        }
    }

    /// Returns `Timestamp.relative` (relative to video start)
    /// as `time::Duration`.
    pub fn to_relative(&self) -> time::Duration {
        time::Duration::milliseconds(self.relative as i64)
    }
    
    /// Returns `Timestamp.duration` (duration of current DEVC chunk)
    /// as `time::Duration`.
    pub fn to_duration(&self) -> time::Duration {
        time::Duration::milliseconds(self.duration as i64)
    }
    
    /// Adds one `Timestamp` to another and returns the resulting `Timestamp`.
    /// Only modifies the `relative` field.
    pub fn add(&self, timestamp: &Self) -> Self {
        Self {
            relative: self.relative + timestamp.relative,
            ..self.to_owned()
        }
    }

    /// Substracts one `Timestamp` from another and returns the resulting `Timestamp`.
    /// Only modifies the `relative` field.
    pub fn sub(&self, timestamp: &Self) -> Self {
        Self {
            relative: self.relative - timestamp.relative,
            ..self.to_owned()
        }
    }
}