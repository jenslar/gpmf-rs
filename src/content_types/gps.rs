use time::{PrimitiveDateTime, Time, ext::NumericalDuration, macros::{datetime, date}};

use crate::{
    FourCC,
    GpmfError,
    Stream,
    Timestamp, content_types::gps
};

use super::primitivedatetime_to_string;

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

    /// Returns the start of the GPMF stream as a date time object.
    /// If no coordinates were logged `None` is returned.
    pub fn t0(&self) -> Option<PrimitiveDateTime> {
        let first_point = self.first()?.to_owned();

        Some(
            // subtract timestamp relative to video timeline from datetime
            first_point.datetime
            - time::Duration::milliseconds(first_point.time?.relative as i64)
        )
    }

    /// Returns the start of the GPMF stream as an ISO8601 formatted string.
    /// If no coordinates were logged `None` is returned.
    pub fn t0_as_string(&self) -> Option<String> {
        self.t0()
            .and_then(|t| primitivedatetime_to_string(&t).ok())
        }
        
    pub fn t_last_as_string(&self) -> Option<String> {
        self.last()
            .and_then(|p| primitivedatetime_to_string(&p.datetime).ok())
    }

    /// Filter points on GPS fix, i.e. the number of satellites
    /// the GPS is locked on to. If satellite lock is not acquired,
    /// the device will log zeros or latest known location with a
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
    /// For Hero10 and earliers (`GPS5` devices) this is logged in `GPSP`
    /// which is DOPx100. A value value below 500 is good
    /// according to <https://github.com/gopro/gpmf-parser>.
    /// For Hero11 an later (`GPS9` devices) DOP is logged in `GPS9`
    pub fn filter(&self, min_gps_fix: u32, min_dop: Option<f64>) -> Self {
        // GoPro has four levels: 0, 2, 3 (No lock, 2D lock, 3D lock)
        let filtered = self.0.iter()
            .filter(|p| 
                match p.fix {
                    Some(f) => f >= min_gps_fix,
                    None => false
                })
            .cloned()
            .collect::<Vec<_>>();
        Self(filtered)
    }

    // pub fn filter(&self, start_ms: u64, end_ms: u64) -> Option<Self> {
    //     let mut points: Vec<Point> = Vec::new();

    //     for point in points.into_iter() {
    //         let t = point.time.as_ref()?;
    //         let start = t.to_relative().num_milliseconds();
    //         let end = start + t.to_duration().num_milliseconds();

    //         if start_ms >= start as u64 && end_ms <= end as u64 {
    //             // points.push(point.to_owned());
    //         }
    //     }

    //     Some(Gps(points))
    // }
}

/// Point derived from GPS data stream.
#[derive(Debug, Clone)]
pub struct GoProPoint {
    /// Latitude.
    pub latitude: f64,
    /// Longitude.
    pub longitude: f64,
    /// Altitude.
    pub altitude: f64,
    /// 2D speed.
    pub speed2d: f64,
    /// 3D speed.
    pub speed3d: f64,
    // /// Heading 0-360 degrees
    // pub heading: f64,
    /// Datetime derived from `GPSU` message.
    pub datetime: PrimitiveDateTime,
    // pub fix: Option<f64>,
    /// DOP, dilution of precision.
    /// `GPSP` for `GPS5` device (Hero10 and earlier),
    /// Value at index 7 in `GPS9` array (Hero11 and later)
    /// A parsed value below 0.5 is good according
    /// to GPMF docs.
    pub dop: Option<f64>,
    /// GPSF for GPS5 device (Hero10 and earlier),
    /// Value nr 9 in GPS9 array (Hero11 and later)
    pub fix: Option<u32>,
    /// Timestamp
    pub time: Option<Timestamp>,
}

impl Default for GoProPoint {
    fn default() -> Self {
        Self { 
            latitude: f64::default(),
            longitude: f64::default(),
            altitude: f64::default(),
            speed2d: f64::default(),
            speed3d: f64::default(),
            datetime: datetime!(2000-01-01 0:00), // GoPro start date
            dop: None,
            fix: None,
            time: None
        }
    }
}

impl std::fmt::Display for GoProPoint {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "\
            latitude:  {}
            longitude: {}
            altitude:  {}
            speed2d:   {}
            speed3d:   {}
            datetime:  {:?}
            fix:       {:?}
            precision: {:?}
            time:      {:?}",
            self.latitude,
            self.longitude,
            self.altitude,
            self.speed2d,
            self.speed3d,
            // self.heading,
            self.datetime,
            self.dop,
            self.fix,
            self.time,
        )
    }
}

// impl From<(&[f64], &[f64])> for Vec<GoProPoint> {
//     /// Convert a GPS9 or GPS5 array and a scale array
//     /// to `GoProPoint`.
//     /// Expects order to be `(GPS, SCALE)`.
//     fn from(value: (&[f64], &[f64])) -> Vec<Self> {
        
//         Vec::new()
//     }
// }

/// Point derived from GPS STRM with STNM "GPS (Lat., Long., Alt., 2D speed, 3D speed)"
impl GoProPoint {
    /// Generates a point from two slices, one containing raw GPS data
    /// from either a `GPS5` or a `GPS9` cluster, the other scale
    /// values.
    /// 
    /// For `GPS5` devices `dop` (dilution of precision) is stored in `GPSP`,
    /// and `fix` in `GPSF` and have to be specified separately
    fn from_raw(
        gps_slice: &[f64],
        scale_slice: &[f64],
        devc_timestamp: Option<Timestamp>,
        datetime: Option<PrimitiveDateTime>,
        dop: Option<u16>,
        fix: Option<u32>,
    ) -> Self {
        assert_eq!(gps_slice.len(), scale_slice.len(),
            "Must be equal: GPS5/9 has length {}, but scale slice has length {}",
            gps_slice.len(),
            scale_slice.len()
        );

        let mut point = Self::default();
        gps_slice.iter().zip(scale_slice)
            .enumerate()
            .for_each(|(i, (gps, scl))| {
                match i {
                    0 => point.latitude = gps/scl,
                    1 => point.longitude = gps/scl,
                    2 => point.altitude = gps/scl,
                    3 => point.speed2d = gps/scl,
                    4 => point.speed3d = gps/scl,
                    // Below values are only valid for GPS9 devices
                    5 => point.datetime += (gps/scl).days(),
                    6 => point.datetime += (gps/scl).seconds(),
                    7 => point.dop = Some(gps/scl),
                    8 => point.fix = Some((gps/scl).round() as u32),
                    _ => (), // break?
                }
            });

        // GPS9 timestamp estimate
        if let Some(ts) = devc_timestamp {
            point.time = Some(ts);
        }

        // Add optional values, only for GPS5 devices
        if let Some(dt) = datetime {
            point.datetime = dt
        }
        if let Some(val) = fix {
            point.fix = Some(val)
        }
        if let Some(val) = dop {
            // specified as DOPx100 for GPS5 devices
            point.dop = Some(val as f64 / 100.)
        }

        point
    }

    /// For Hero10 and earlier models.
    /// 
    /// Parse stream of type `STRM` with name (`STNM`) "GPS (Lat., Long., Alt., 2D speed, 3D speed)",
    /// which contains a coordinate cluster.
    /// Returns a single point from a linear average of values within a single GPS stream, lumped together
    /// once/second (only a single timestamp is logged for each cluster).
    /// GoPro GPS logs at 10 or 18Hz (depending on model) so on average 10 or 18 points are logged each second.
    /// For those who record while moving at very high velocities, a latitude dependent average could
    /// be implemented in a future release.
    pub fn from_gps5(devc_stream: &Stream) -> Option<Self> {
        // REQUIRED, each Vec<f64>, logged coordinates as cluster: [lat, lon, alt, 2d speed, 3d speed]
        // On average 18 coordinates per GPS5 message.
        let gps5 = devc_stream
            .find(&FourCC::GPS5)
            .and_then(|s| s.to_vec_f64())?;

        let mut lat_sum: f64 = 0.0;
        let mut lon_sum: f64 = 0.0;
        let mut alt_sum: f64 = 0.0;
        let mut sp2d_sum: f64 = 0.0;
        let mut sp3d_sum: f64 = 0.0;

        // // let mut gps5_count: usize = 0;

        let len = gps5.len();

        gps5.iter().for_each(|v| {
            // gps5.iter().enumerate().for_each(|(i, v)| {
            //     gps5_count = i + 1; // should be equal to len of corresponding vec
            lat_sum += v[0];
            lon_sum += v[1];
            alt_sum += v[2];
            sp2d_sum += v[3];
            sp3d_sum += v[4];
        });

        // REQUIRED
        let scale = devc_stream
            .find(&FourCC::SCAL)
            .and_then(|s| s.to_f64())?;

        // all set to 1.0 to avoid div by 0
        let mut lat_scl: f64 = 1.0;
        let mut lon_scl: f64 = 1.0;
        let mut alt_scl: f64 = 1.0;
        let mut sp2d_scl: f64 = 1.0;
        let mut sp3d_scl: f64 = 1.0;

        // REQUIRED, 5 single-value BaseTypes, each a scale divisor for the
        // corresponding raw value in GPS5. Order is the same as for GPS5:
        // the first scale value should be applied to first value in a single GPS5
        // BaseType vec (latitude), the second to the second GPS5 value (longitude) and so on.
        scale.iter().enumerate().for_each(|(i, &s)| {
            match i {
                0 => lat_scl = s,
                1 => lon_scl = s,
                2 => alt_scl = s,
                3 => sp2d_scl = s,
                4 => sp3d_scl = s,
                _ => (), // i > 4 should not exist, check? break?
            }
        });

        // OPTIONAL (is it...?), timestamp for coordinate cluster
        let gpsu: PrimitiveDateTime = devc_stream
            .find(&FourCC::GPSU)
                .and_then(|s| s.first_value())
                .and_then(|v| v.into())?;
        // or return generic date than error if it's only timestamp that can not be parsed then use:
        // .unwrap_or(NaiveDate::from_ymd(2000, 1, 1)
        // .and_hms_milli(0, 0, 0, 0)),

        // OPTIONAL, GPS fix
        let gpsf: Option<u32> = devc_stream
        // let gpsf: Option<u64> = stream
            .find(&FourCC::GPSF)
                .and_then(|s| s.first_value())
                .and_then(|v| v.into()); // GPS Fix Hero 7, 9 confirmed

        // OPTIONAL, GPS precision
        let gpsp: Option<u16> = devc_stream
        // let gpsp: Option<u64> = stream
            .find(&FourCC::GPSP)
                .and_then(|s| s.first_value())
                .and_then(|v| v.into()); // GPS Precision Hero 7, 9 confirmed

        Some(Self {
            latitude: lat_sum / len as f64 / lat_scl,
            longitude: lon_sum / len as f64 / lon_scl,
            altitude: alt_sum / len as f64 / alt_scl,
            speed2d: sp2d_sum / len as f64 / sp2d_scl,
            speed3d: sp3d_sum / len as f64 / sp3d_scl,
            datetime: gpsu,
            time: devc_stream.time.to_owned(),
            dop: gpsp.map(|p| p as f64 / 100.),
            fix: gpsf,
        })
    }

    /// For Hero11 and later models.
    /// 
    /// Parse stream of type `STRM` with `STNM` "GPS (Lat., Long., Alt., 2D, 3D, days, secs, DOP, fix)"
    /// 
    /// Since 
    pub fn from_gps9(devc_stream: &Stream) -> Option<Vec<Self>> {
        // REQUIRED, each Vec<f64>, logged coordinates as cluster: [lat, lon, alt, 2d speed, 3d speed]
        // On average 18 coordinates per GPS5 message.
        // 230323 added into() for Value::Complex -> Vec<f64>
        let gps9 = devc_stream
            .find(&FourCC::GPS9)
            .and_then(|s| s.to_vec_f64())?;

        let len = gps9.len();

        // REQUIRED
        let scale = devc_stream
            .find(&FourCC::SCAL)
            .and_then(|s| s.to_f64())?;

        let points = gps9.iter()
            .enumerate()
            .map(|(i, vec)| {
                let ts = devc_stream.time.as_ref().map(|t| Timestamp {
                    relative: (t.relative as f64 + i as f64 * t.duration as f64 / len as f64).round() as u32,
                    duration: (t.duration as f64 / len as f64).round() as u32
                });
                GoProPoint::from_raw(&vec, &scale, ts, None, None, None)
            })
            .collect::<Vec<_>>();

        Some(points)
    }

    pub fn datetime_to_string(&self) -> Result<String, GpmfError> {
        primitivedatetime_to_string(&self.datetime)
    }
}
