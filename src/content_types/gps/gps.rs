use core::f64;
use std::u32;

use time::{Duration, PrimitiveDateTime};
use crate::content_types::primitivedatetime_to_string;

use super::GoProPoint;

/// Gps point cluster, converted from `GPS5` or `GPS9`.
#[derive(Debug, Default, Clone)]
pub struct Gps(pub Vec<GoProPoint>);

impl Gps {
    pub fn len(&self) -> usize {
        self.0.len()
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
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

    /// Returns center of GPS points cluster.
    pub fn center(&self) -> Option<GoProPoint> {
        points_average(&self.0)
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
    /// derived from the number of satellites the GPS is locked on to.
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
    /// `min_dop` corresponds to [dilution of precision](https://en.wikipedia.org/wiki/Dilution_of_precision_(navigation)).
    /// For Hero10 and earlier (`GPS5` devices) this is logged in `GPSP`.
    /// For Hero11 an later (`GPS9` devices) DOP is logged in `GPS9`.
    /// A value value below 500 is good
    /// according to <https://github.com/gopro/gpmf-parser>.
    pub fn prune(self, min_fix: Option<u32>, max_dop: Option<f64>) -> Self {
        // GoPro has four levels: 0, 2, 3 (No lock, 2D lock, 3D lock)
        let fix = min_fix.unwrap_or(u32::MIN); // set to 0 to let all pass through
        let dop = max_dop.unwrap_or(f64::MAX); // set to MAX/+INF to let all pass through
        Self(
            self.0
                .into_iter()
                .filter(|p| p.dop <= dop && p.fix >= fix)
                .collect::<Vec<_>>(),
        )
    }

    /// Prune points mutably if `gps_fix_min` is below specified value,
    /// derived from the number of satellites the GPS is locked on to,
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
    /// `min_dop` corresponds to [dilution of precision](https://en.wikipedia.org/wiki/Dilution_of_precision_(navigation)).
    /// For Hero10 and earlier (`GPS5` devices) this is logged in `GPSP`.
    /// For Hero11 an later (`GPS9` devices) DOP is logged in `GPS9`
    /// (Hero12 does not have a GPS module, Hero 13 again includes one).
    /// A value below 5 (unscaled GPMF value of 500) is good.
    /// See <https://github.com/gopro/gpmf-parser>.
    pub fn prune_mut(&mut self, min_fix: Option<u32>, max_dop: Option<f64>) -> usize {
        let len1 = self.len();
        let fix = min_fix.unwrap_or(u32::MIN); // set to 0 to let all pass through
        let dop = max_dop.unwrap_or(f64::MAX); // set to MAX/+INF to let all pass through
        self.0.retain(|p| p.dop <= dop && p.fix >= fix);
        let len2 = self.len();
        return len1 - len2;
    }
    fn prune_mut_old(&mut self, min_fix: u32, max_dop: Option<f64>) -> usize {
        let len1 = self.len();
        self.0.retain(|p| match max_dop {
            Some(dop) => {
                (p.dop < dop) && (p.fix >= min_fix)
            }
            None => p.fix >= min_fix
        });
        let len2 = self.len();
        return len1 - len2;
    }
}

/// Returns a latitude dependent average coordinate for specified points.
pub(crate) fn points_average(points: &[GoProPoint]) -> Option<GoProPoint> {
    // see: https://carto.com/blog/center-of-points/ NO LONGER UP
    // atan2(y,x) where y = sum((sin(yi)+...+sin(yn))/n), x = sum((cos(xi)+...cos(xn))/n), y, i in radians

    let dur_total: Duration = points.iter().map(|p| p.time).sum();
    // let datetime_first = points.first().map(|p| p.datetime)?;
    // let datetime_last = points.last().map(|p| p.datetime)?;
    // let datetime_avg: PrimitiveDateTime = match points.len() {
    //     1 => datetime_first,
    //     _ => datetime_first + Duration::seconds_f64((datetime_last - datetime_first).as_seconds_f64() / 2.)
    // };

    let deg2rad = std::f64::consts::PI / 180.0; // inverse for radians to degress

    let mut lon_rad_sin: Vec<f64> = Vec::new(); // sin values
    let mut lon_rad_cos: Vec<f64> = Vec::new(); // cos values
    let mut lat_rad: Vec<f64> = Vec::new(); // arithmetic average ok
    let mut alt: Vec<f64> = Vec::new(); // arithmetic average ok
    let mut sp2d: Vec<f64> = Vec::new(); // arithmetic average ok
    let mut sp3d: Vec<f64> = Vec::new(); // arithmetic average ok
    let mut dop: Vec<f64> = Vec::new();
    let mut fix: Vec<f64> = Vec::new();

    for pt in points.iter() {
        lon_rad_sin.push((pt.longitude * deg2rad).sin());
        lon_rad_cos.push((pt.longitude * deg2rad).cos());
        lat_rad.push(pt.latitude * deg2rad); // arithmetic avg ok, only converts to radians
        alt.push(pt.altitude);
        // magnetometer is MAX cameras only
        // if let Some(h) = pt.heading {
        //     hdg.push(h)
        // }
        sp2d.push(pt.speed2d);
        sp3d.push(pt.speed3d);
        dop.push(pt.dop);
        fix.push(pt.fix as f64);
    }

    // AVERAGING LATITUDE DEPENDENT LONGITUDES
    let lon_rad_sin_sum = average(&lon_rad_sin);
    let lon_rad_cos_sum = average(&lon_rad_cos);
    let lon_avg_deg = f64::atan2(lon_rad_sin_sum, lon_rad_cos_sum) / deg2rad; // -> degrees
    let lat_avg_deg = average(&lat_rad) / deg2rad; // -> degrees
    let alt_avg = average(&alt);
    // magnetometer is MAX cameras only
    // let hdg_avg = match hdg.is_empty() {
    //     true => None,
    //     false => Some(average(&hdg)),
    // };
    let sp2d_avg = average(&sp2d);
    let sp3d_avg = average(&sp3d);
    let dop_avg = average(&dop);
    let fix_avg = average(&fix);

    Some(GoProPoint {
        latitude: lat_avg_deg,
        longitude: lon_avg_deg,
        altitude: alt_avg,
        // heading: hdg_avg,
        speed2d: sp2d_avg,
        speed3d: sp3d_avg,
        // Use datetime for first point in cluster to represent the start
        // of the timestamp for averaged points. (rather than average datetime)
        datetime: points.first().map(|p| p.datetime)?,
        // datetime: datetime_avg,
        // timestamp: should be start of first point not average,
        // so that timestamp + duration = timespan within which all averaged points were logged
        // timestamp: ts_first, // TODO test! hero11 then virb (remove set_timedelta for virb)
        // timestamp: Some(time_avg), // OLD
        // duration: should be sum of all durations
        // so that timestamp + duration = timespan within which all averaged points were logged
        time: dur_total, // TODO test! hero11 then virb (remove set_timedelta for virb)
        // duration: points.first().and_then(|p| p.duration), // OLD
        // description,
        dop: dop_avg,
        fix: fix_avg as u32 // meaningless but eh...
    })
}

fn average(nums: &[f64]) -> f64 {
    nums.iter().sum::<f64>() / nums.len() as f64
}