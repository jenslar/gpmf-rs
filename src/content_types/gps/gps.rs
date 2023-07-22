use time::PrimitiveDateTime;
use crate::content_types::primitivedatetime_to_string;

use super::GoProPoint;

/// Gps point cluster, converted from `GPS5` or `GPS9`.
#[derive(Debug, Default, Clone)]
pub struct Gps(pub Vec<GoProPoint>);

impl Gps {
    pub fn len(&self) -> usize {
        self.0.len()
    }

    pub fn iter(&self) -> impl Iterator<Item = &GoProPoint> {
        self.0.iter()
    }

    pub fn iter_mut(&mut self) -> impl Iterator<Item = &mut GoProPoint> {
        self.0.iter_mut()
    }

    pub fn into_iter(self) -> impl Iterator<Item = GoProPoint> {
        self.0.into_iter()
    }

    pub fn first(&self) -> Option<&GoProPoint> {
        self.0.first()
    }

    pub fn last(&self) -> Option<&GoProPoint> {
        self.0.last()
    }

    // pub fn first_timestamp(&self) -> Option<&Timestamp> {
    //     self.0.first().and_then(|p| p.time.as_ref())
    // }

    // pub fn last_timestamp(&self) -> Option<&Timestamp> {
    //     self.0.last().and_then(|p| p.time.as_ref())
    // }

    /// Returns the start of the GPMF stream as `PrimitiveDateTime`.
    /// Returns `None` if no points were logged or if no points with minimum
    /// level of satellite lock were logged. Defaults to 2D lock if `min_gps_fix` is `None`.
    pub fn t0(&self, min_gps_fix: Option<u32>) -> Option<PrimitiveDateTime> {
        let first_point = self
            .iter()
            .find(|p| p.fix >= min_gps_fix.unwrap_or(2))? // find first with satellite lock
            .to_owned();

        Some(
            // subtract timestamp relative to video timeline from datetime
            first_point.datetime - first_point.time,
        )
    }

    /// Returns the start of the GPMF stream as an ISO8601 formatted string.
    /// Returns `None` if no points were logged or if no points with minimum
    /// level of satellite lock were logged. Defaults to 3D lock if `min_gps_fix` is `None`.
    pub fn t0_as_string(&self, min_gps_fix: Option<u32>) -> Option<String> {
        self.t0(min_gps_fix)
            .and_then(|t| primitivedatetime_to_string(&t).ok())
    }

    pub fn t_last_as_string(&self) -> Option<String> {
        self.last()
            .and_then(|p| primitivedatetime_to_string(&p.datetime).ok())
    }

    /// Prune points if `gps_fix_min` is below specified value,
    /// i.e. the number of satellites the GPS is locked on to.
    /// If satellite lock is not acquired,
    /// the device will log zeros or possibly latest known location with a
    /// GPS fix of `0`, meaning both time and location will be
    /// wrong.
    ///
    /// `min_gps_fix` corresponds to satellite lock and should be
    /// at least 2 to ensure returned points have logged a position
    /// that is in the vicinity of the camera.
    /// Valid values are 0 (no lock), 2 (2D lock), 3 (3D lock).
    /// On Hero 10 and earlier (devices that use `GPS5`) this is logged
    /// in `GPSF`. Hero11 and later deprecate `GPS5` the value in GPS9
    /// should be used instead.
    ///
    /// `min_dop` corresponds to [dilution of position](https://en.wikipedia.org/wiki/Dilution_of_precision_(navigation)).
    /// For Hero10 and earlier (`GPS5` devices) this is logged in `GPSP`.
    /// For Hero11 an later (`GPS9` devices) DOP is logged in `GPS9`.
    /// A value value below 500 is good
    /// according to <https://github.com/gopro/gpmf-parser>.
    pub fn prune(self, min_gps_fix: u32, _min_dop: Option<f64>) -> Self {
        // GoPro has four levels: 0, 2, 3 (No lock, 2D lock, 3D lock)
        Self(
            self.0
                .into_iter()
                .filter(|p| p.fix >= min_gps_fix)
                .collect::<Vec<_>>(),
        )
    }

    /// Prune points mutably if `gps_fix_min` is below specified value,
    /// i.e. the number of satellites the GPS is locked on to,
    /// and returns the number of points pruned.
    /// If satellite lock is not acquired,
    /// the device will log zeros or possibly latest known location with a
    /// GPS fix of `0`, meaning both time and location will be
    /// wrong.
    ///
    /// `min_gps_fix` corresponds to satellite lock and should be
    /// at least 2 to ensure returned points have logged a position
    /// that is in the vicinity of the camera.
    /// 
    /// Valid values are:
    /// - 0 (no lock)
    /// - 2 (2D lock)
    /// - 3 (3D lock)
    /// 
    /// On Hero 10 and earlier (devices that use `GPS5`) this is logged
    /// in `GPSF`. Hero11 and later deprecate `GPS5` the value in GPS9
    /// should be used instead.
    ///
    /// `min_dop` corresponds to [dilution of position](https://en.wikipedia.org/wiki/Dilution_of_precision_(navigation)).
    /// For Hero10 and earlier (`GPS5` devices) this is logged in `GPSP`.
    /// For Hero11 an later (`GPS9` devices) DOP is logged in `GPS9`.
    /// A value below 500 is good
    /// according to <https://github.com/gopro/gpmf-parser>.
    pub fn prune_mut(&mut self, min_gps_fix: u32, _min_dop: Option<f64>) -> usize {
        let len1 = self.len();
        self.0.retain(|p| p.fix >= min_gps_fix);
        let len2 = self.len();
        return len1 - len2;
    }
}
