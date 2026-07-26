use core::f64;
use std::u32;

use log::warn;
use spatio_types::point::TemporalPoint3D;
use time::{Duration, OffsetDateTime};
use crate::content_types::{
    offsetdatetime_to_string,
};

use super::GoProPoint;

/// Gps point cluster, converted from `GPS5` or `GPS9`.
#[derive(Debug, Default, Clone, PartialEq)]
pub struct Gps(Vec<GoProPoint>);

#[cfg(feature = "gpx")]
impl From<Gps> for gpx::Gpx {
    fn from(value: Gps) -> Self {
        value.to_gpx()
    }
}

impl Gps {
    pub fn new(points: Vec<GoProPoint>) -> Self {
        Self(points)
    }

    pub fn points(&self) -> &[GoProPoint] {
        &self.0
    }

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
        average_points(&self.0)
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
    // pub fn t0(&self, min_gps_fix: Option<u32>) -> Option<PrimitiveDateTime> {
    pub fn t0(&self, min_gps_fix: Option<u32>) -> Option<OffsetDateTime> {
        let first_point = self
            .iter()
            .find(|p| p.fix >= min_gps_fix.unwrap_or(2))? // find first with satellite lock
            .to_owned();

        Some(
            // subtract timestamp relative to video timeline from datetime
            first_point.datetime().ok()? - first_point.timestamp,
        )
    }

    /// Returns the start of the GPMF stream as an ISO8601 formatted string.
    /// Returns `None` if no points were logged or if no points with minimum
    /// level of satellite lock were logged. Defaults to 3D lock if `min_gps_fix` is `None`.
    pub fn t0_as_string(&self, min_gps_fix: Option<u32>) -> Option<String> {
        self.t0(min_gps_fix)
            .and_then(|t| offsetdatetime_to_string(&t).ok())
    }

    pub fn t_last_as_string(&self) -> Option<String> {
        self.last()
            .and_then(|p| offsetdatetime_to_string(&p.datetime().ok()?).ok())
    }

    /// Prune points if `min_satellite_lock` is below specified value,
    /// or above `max_dilution_of_precision`.
    ///
    /// If satellite lock is not acquired,
    /// the device will log zeros or possibly latest known location with a
    /// GPS fix of `0`, meaning both time and location will be
    /// unusable.
    ///
    /// `min_satellite_lock` corresponds to satellite lock threshold and should be
    /// at least 2 to ensure returned points have logged a position
    /// that is in the vicinity of the camera.
    ///
    /// Valid values are:
    /// - 0 (no lock)
    /// - 2 (2D lock)
    /// - 3 (3D lock)
    ///
    /// On Hero 10 and earlier (`GPS5` devices) this is logged
    /// in `GPSF`. For Hero11 and later the value in `GPS9`
    /// should be used.
    ///
    /// `max_dilution_of_precision` corresponds to [dilution of precision](https://en.wikipedia.org/wiki/Dilution_of_precision_(navigation)) threshold.
    /// For Hero10 and earlier (`GPS5` devices) this is logged in `GPSP`.
    /// For Hero11 and later the value in `GPS9` should be used.
    /// A value value below 5 (or unscaled 500) can be considered good.
    pub fn prune(
        self,
        min_satellite_lock: Option<u32>,
        max_dilution_of_precision: Option<f64>
    ) -> Self {
        // GoPro has four levels: 0, 2, 3 (No lock, 2D lock, 3D lock)
        let min_lock = min_satellite_lock.unwrap_or(u32::MIN); // set to 0 to let all pass through
        let max_dop = max_dilution_of_precision.unwrap_or(f64::MAX); // set to MAX/+INF to let all pass through
        Self(
            self.0
                .into_iter()
                .filter(|p| p.dop <= max_dop && p.fix >= min_lock)
                .collect::<Vec<_>>(),
        )
    }

    /// Prune points mutably if `min_satellite_lock` is below specified value,
    /// or above `max_dilution_of_precision`. Returns number of pruned points.
    ///
    /// If satellite lock is not acquired,
    /// the device will log zeros or possibly latest known location with a
    /// GPS fix of `0`, meaning both time and location will be
    /// unusable.
    ///
    /// `min_satellite_lock` corresponds to satellite lock threshold and should be
    /// at least 2 to ensure returned points have logged a position
    /// that is in the vicinity of the camera.
    ///
    /// Valid values are:
    /// - 0 (no lock)
    /// - 2 (2D lock)
    /// - 3 (3D lock)
    ///
    /// On Hero 10 and earlier (`GPS5` devices) this is logged
    /// in `GPSF`. For Hero11 and later the value in `GPS9`
    /// should be used.
    ///
    /// `max_dilution_of_precision` corresponds to [dilution of precision](https://en.wikipedia.org/wiki/Dilution_of_precision_(navigation)) threshold.
    /// For Hero10 and earlier (`GPS5` devices) this is logged in `GPSP`.
    /// For Hero11 and later the value in `GPS9` should be used.
    /// A value value below 5 (or unscaled 500) can be considered good.
    pub fn prune_mut(&mut self, min_fix: Option<u32>, max_dop: Option<f64>) -> usize {
        let len1 = self.len();
        let fix = min_fix.unwrap_or(u32::MIN); // set to 0 to let all pass through
        let dop = max_dop.unwrap_or(f64::MAX); // set to MAX/+INF to let all pass through
        self.0.retain(|p| p.dop <= dop && p.fix >= fix);
        let len2 = self.len();
        return len1 - len2;
    }

    /// Returns tuples representing 2D
    /// bounding box.
    fn bounds(&self) -> Option<[(f64, f64); 4]> {
        if !self.is_empty() {
            let mut lat_min = self.first().map(|p| p.latitude())?;
            let mut lat_max = self.first().map(|p| p.latitude())?;
            let mut lon_min = self.first().map(|p| p.longitude())?;
            let mut lon_max = self.first().map(|p| p.longitude())?;

            self.iter().skip(1)
                .for_each(|p| {
                    if p.latitude() > lat_max {
                        lat_max = p.latitude()
                    }
                    if p.latitude() < lat_min {
                        lat_min = p.latitude()
                    }
                    if p.longitude() > lon_max {
                        lon_max = p.longitude()
                    }
                    if p.longitude() < lon_min {
                        lon_min = p.longitude()
                    }
                });

            return Some([
                (lat_min, lon_min),
                (lat_min, lon_max),
                (lat_max, lon_min),
                (lat_max, lon_max),
            ])
        }
        None
    }

    /// Downsample points. Latitude dependent downsampling.
    /// For 1000 points a `downsample_factor = 100` results in
    /// 10 points and so on.
    /// `min` is the minimum nuber of points left
    /// after downsampling.
    /// E.g. constructing a polyline later requires a minimum of
    /// two points.
    /// At least one point will be returned regardless of `downsample_factor`.
    pub fn downsample(
        self,
        downsample_factor: usize,
        min: Option<usize>
    ) -> Self {
        Self(downsample_points(self.points(), downsample_factor, min))
    }

    /// Mutably downsample points. Latitude dependent downsampling.
    /// For 1000 points a `downsample_factor = 100` results in
    /// 10 points and so on.
    /// `min` is the minimum nuber of points left
    /// after downsampling.
    /// E.g. constructing a polyline later requires a minimum of
    /// two points.
    /// At least one point will be returned regardless of `downsample_factor`.
    pub fn downsample_mut(
        &mut self,
        downsample_factor: usize,
        min: Option<usize>
    ) {
        self.0 = downsample_points(&self.points(), downsample_factor, min);
    }

    #[cfg(feature = "gpx")]
    /// Simple export to GPX 1.1.
    pub fn to_gpx(&self) -> gpx::Gpx {
        let mut track = gpx::Track::new();
        let points: Vec<gpx::Waypoint> = self
            .points()
            .iter()
            .map(|p| gpx::Waypoint::from(p))
            .collect();
        track.segments = vec![gpx::TrackSegment {points}];
        let mut gpxout = gpx::Gpx::default();
        gpxout.version = gpx::GpxVersion::Gpx11;
        gpxout.tracks = vec![track];

        gpxout
    }

    #[cfg(feature = "gpx")]
    /// Write GPX 1.1 file.
    pub fn write_gpx(&self, path: &std::path::Path) -> Result<(), crate::GpmfError> {
        let gpxout = self.to_gpx();
        let mut file = std::fs::File::create_new(path)?;

        Ok(gpx::write(&gpxout, &mut file)?)
    }
}

/// Downsample points.
/// Clusters points in sizes equal to `sample_factor`,
/// then downsamples each sub-cluster to a single point.
/// Optionally set a minimum number of points to return via `min`.
/// If `sample_factor` results in fewer points than `min`,
/// `min` will be used in its place.
pub fn downsample_points(
    points: &[GoProPoint],
    mut sample_factor: usize,
    min: Option<usize>,
) -> Vec<GoProPoint> {
    // points may be empty due to being filtered out
    // (satellite lock level too low, or DOP too high for GoPro)
    if points.is_empty() {
        return Vec::new();
    }

    match sample_factor {
        // avoid division by 0
        0 => {
            warn!("Sample factor cannot be 0. Skipping downsample.");
            return points.to_vec()
        },
        1 => return points.to_vec(),
        // ensure downsampling will at lest yield a single point
        f if f > points.len() => sample_factor = points.len(),
        _ => (),
    }

    // Int division for checking if downsample factor
    // causes fewer than optionally set min number of points
    if let Some(m) = min {
        if (points.len() / sample_factor) < m {
            // div_ceil will be in upcoming rust version:
            // https://github.com/rust-lang/rfcs/issues/2844
            // sample_factor = points.len().div_ceil(m);
            // sample_factor = (points.len() as f64 / m as f64).ceil() as usize // should this be .floor()?
            // 220914 changed to .floor()
            sample_factor = (points.len() as f64 / m as f64).floor() as usize // .floor() IS UNTESTED
        }
    }

    points.chunks(sample_factor)
        .filter_map(|c| average_points(c))
        .collect::<Vec<_>>()
}

/// Returns a latitude dependent average coordinate for specified points.
pub(crate) fn average_points(points: &[GoProPoint]) -> Option<GoProPoint> {
    // see: https://carto.com/blog/center-of-points/ NO LONGER UP
    // atan2(y,x) where y = sum((sin(yi)+...+sin(yn))/n), x = sum((cos(xi)+...cos(xn))/n), y, i in radians

    let dur_total: Duration = points.iter().map(|p| p.timestamp).sum();

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
        lon_rad_sin.push((pt.longitude() * deg2rad).sin());
        lon_rad_cos.push((pt.longitude() * deg2rad).cos());
        lat_rad.push(pt.latitude() * deg2rad); // arithmetic avg ok, only converts to radians
        alt.push(pt.altitude());
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

    let point2d = spatio_types::geo::Point::new(lon_avg_deg, lat_avg_deg);
    let systemtime = points.first()?.systemtime();
    let point_xyzt = TemporalPoint3D::new(point2d, alt_avg, systemtime.to_owned());

    Some(GoProPoint {
        point: point_xyzt,
        speed2d: sp2d_avg,
        speed3d: sp3d_avg,
        // sum of all relative "video" timestamp durations
        // so that timestamp + duration = timespan within which all averaged points were logged
        timestamp: dur_total, // TODO test! hero11
        dop: dop_avg,
        fix: fix_avg as u32 // meaningless but eh...
    })
}

fn average(nums: &[f64]) -> f64 {
    nums.iter().sum::<f64>() / nums.len() as f64
}
