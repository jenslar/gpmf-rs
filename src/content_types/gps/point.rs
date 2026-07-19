use std::time::{SystemTime, UNIX_EPOCH};

use spatio_types::{geo::Point, point::TemporalPoint3D};
use time::{Duration, OffsetDateTime, PrimitiveDateTime, ext::NumericalDuration, macros::datetime};
use crate::{FourCC, Stream, GpmfError, content_types::primitivedatetime_to_string};

/// Point derived from GPS data stream.
#[derive(Debug, Clone, PartialEq)]
pub struct GoProPoint {
    pub(crate) point: TemporalPoint3D,
    /// 2D speed.
    pub speed2d: f64,
    /// 3D speed.
    pub speed3d: f64,
    /// Datetime.
    /// - GPS5 devices: derived from `GPSU` message, once per point cluster.
    /// - GPS9 devices: logged per point.
    // pub datetime: PrimitiveDateTime,
    /// DOP, dilution of precision.
    /// `GPSP` for `GPS5` device (Hero10 and earlier),
    /// Value at index 7 in `GPS9` array (Hero11 and later)
    /// A parsed value below 0.5 is good according
    /// to GPMF docs.
    pub dop: f64,
    /// GPSF for GPS5 device (Hero10 and earlier),
    /// Value nr 9 in GPS9 array (Hero11 and later)
    pub fix: u32,
    /// Timestamp relative to video
    pub timestamp: Duration,
}

impl Default for GoProPoint {
    fn default() -> Self {
        Self {
            point: Self::temporal_point_default(),
            speed2d: f64::default(),
            speed3d: f64::default(),
            // datetime: datetime!(2000-01-01 0:00), // GoPro start date
            dop: f64::default(),
            fix: u32::default(),
            timestamp: Duration::default(),
        }
    }
}

#[cfg(feature = "gpx")]
impl From<&GoProPoint> for gpx::Waypoint {
    /// Converts the local `crate::Point` to `Gpx::Waypoint`.
    fn from(value: &GoProPoint) -> gpx::Waypoint {
        let gp = geo_types::Point::new(value.longitude(), value.latitude());
        let mut wp = gpx::Waypoint::new(gp);

        wp.elevation = Some(value.altitude());
        wp.speed = Some(value.speed2d);
        wp.time = value.datetime()
            .map(|dt| dt.into())
            .ok();

        // other GPX fields:
        // name: todo!(), // use for ID?
        // geoidheight: todo!(), // compared to altitude/elevation?
        // Below fields only relevant for GoPro
        // fix: todo!(), //
        // hdop: todo!(), // can these be found somewhere? which is gopros dop type? horisontal, vertical or positional?
        // vdop: todo!(),
        // pdop: todo!(),

        wp
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
            fix:       {}
            precision: {}
            time:      {:?}",
            self.latitude(),
            self.longitude(),
            self.altitude(),
            self.speed2d,
            self.speed3d,
            self.dop,
            self.fix,
            self.timestamp,
        )
    }
}

/// Point derived from GPS STRM with STNM "GPS (Lat., Long., Alt., 2D speed, 3D speed)"
impl GoProPoint {
    /// Generates a point from two slices: one slice contains raw GPS data
    /// from either a `GPS5` (5 value array) or a `GPS9` (9 value array) cluster,
    /// the other slice contains scaling values.
    ///
    /// Slices map 1:1. I.e. data value at index 0 in `gps_slice` should
    /// be scaled with value at index 0 in `scale_slice` and so on.
    ///
    /// For `GPS5` devices `dop` (dilution of precision) is stored in `GPSP`,
    /// and `fix` in `GPSF` and have to be specified separately
    fn from_gps9_slice(
        gps_slice: &[f64],   // GPS5 (all devices) or GPS9 (Hero11)
        scale_slice: &[f64], // SCAL (all devices)
    ) -> Self {
        assert_eq!(gps_slice.len(), scale_slice.len(),
            "Must be equal in size: GPS5/9 slice has length {}, but scale slice has length {}",
            gps_slice.len(),
            scale_slice.len()
        );

        // Default point with time set to GoPro basetime 2000-01-01
        let mut point = Self::default();
        let mut latitude = 0.;
        let mut longitude = 0.;
        // iterate over data and scale slices simultaneously,
        // i.e. data value will match correct scale value
        gps_slice.iter().zip(scale_slice)
            .enumerate()
            .for_each(|(i, (gps, scl))| {
                match i {
                    // 0 - 4 GPS5, GPS9
                    // 0 => point.latitude. = gps/scl,
                    // 1 => point.longitude = gps/scl,
                    0 => latitude = gps/scl, // no direct access to x/y (lon/lat) fields
                    1 => longitude = gps/scl, // no direct access to x/y (lon/lat) fields
                    2 => point.point.altitude = gps/scl,
                    3 => point.speed2d = gps/scl,
                    4 => point.speed3d = gps/scl,
                    // 5 - 8 GPS9 only
                    // 5 => point.datetime += (gps/scl).days(),
                    // 6 => point.datetime += (gps/scl).seconds(),
                    5 => point.point.timestamp += (gps/scl).days(),
                    6 => point.point.timestamp += (gps/scl).seconds(),
                    7 => point.dop = gps/scl,
                    8 => point.fix = (gps/scl).round() as u32,
                    _ => (),
                }
            });

        point.with_lat_lon(latitude, longitude)
    }

    pub(crate) fn temporal_point_default() -> TemporalPoint3D {
        let p2d = spatio_types::geo::Point::new(f64::default(), f64::default());
        let dt_nanos_i128 = datetime!(2000-01-01 0:00 +0).unix_timestamp_nanos(); // using gopro "default" datetime
        let systime = UNIX_EPOCH + Duration::nanoseconds_i128(dt_nanos_i128);
        TemporalPoint3D::new(p2d, f64::default(), systime)
    }

    pub fn point(&self) -> &TemporalPoint3D {
        &self.point
    }

    pub fn latitude(&self) -> f64 {
        self.point.point().lat()
    }

    pub fn longitude(&self) -> f64 {
        self.point.point().lon()
    }

    pub fn altitude(&self) -> f64 {
        self.point.altitude()
    }

    /// Returns haversine/great arc distance in meters to another point.
    pub fn haversine(&self, other: &Self) -> f64 {
        self.point.distance_to(&other.point)
    }

    pub fn add_duration(&mut self, duration: Duration) {
        self.point.timestamp += duration;
    }

    /// Returns date time as system time stamp,
    /// as set in the underlying point type.
    pub fn systemtime(&self) -> &SystemTime {
        self.point.timestamp()
    }

    pub fn datetime(&self) -> Result<OffsetDateTime, GpmfError>{
        let nanoseconds_u128 = self.point
            .timestamp()
            .duration_since(SystemTime::UNIX_EPOCH)?
            .as_nanos();
        let nanoseconds_i128 = i128::try_from(nanoseconds_u128)?;
        Ok(OffsetDateTime::from_unix_timestamp_nanos(nanoseconds_i128)?)
    }

    // // Sets datetime.
    // pub fn with_primitivedatetime(self, datetime: PrimitiveDateTime) -> Self {
    //     Self {
    //         datetime,
    //         ..self
    //     }
    // }

    // pub fn with_system_time(self, system_time: &SystemTime) -> Self {
    //     Self {
    //         point: TemporalPoint3D {
    //             timestamp: system_time.to_owned(),
    //             ..self.point
    //         },
    //         ..self
    //     }
    // }

    // pub fn with_system_time_nanos(self, system_time_nanos: i128) -> Self {
    //     Self {
    //         point: TemporalPoint3D {
    //             timestamp: UNIX_EPOCH + Duration::nanoseconds_i128(system_time_nanos),
    //             ..self.point
    //         },
    //         ..self
    //     }
    // }

    // Sets dilution of precision.
    pub fn with_dop(self, dop: f64) -> Self {
        Self {
            dop,
            ..self
        }
    }

    // Sets GPS fix, i.e. satellite lock,
    // where 0 = no lock, 2 = 2D lock, 3 = 3D lock.
    pub fn with_fix(self, fix: u32) -> Self {
        Self {
            fix,
            ..self
        }
    }

    pub fn with_lat_lon(self, latitude: f64, longitude: f64) -> Self {
        Self {
            point: TemporalPoint3D {
                point: spatio_types::geo::Point::new(longitude, latitude),
                ..self.point
            },
            ..self
        }
    }


    pub fn with_altitude(self, altitude: f64) -> Self {
        Self {
            point: TemporalPoint3D {
                altitude,
                ..self.point
            },
            ..self
        }
    }

    /// For Hero10 and earlier models. These log at 18Hz.
    /// Returns a linear average of the point cluster in the specified DEVC stream.
    /// GPS5 devices log datetime, GPS fix, GPS dop once for the whole cluster.
    ///
    /// Stream name (`STNM`): "GPS (Lat., Long., Alt., 2D speed, 3D speed)".
    ///
    /// Note: For those who record while moving at very high velocities,
    /// a latitude dependent average could be implemented in a future release.
    pub fn from_gps5(devc_stream: &Stream) -> Option<Self> {
        // logged coordinates as cluster: [lat, lon, alt, 2d speed, 3d speed]
        // On average 18 coordinates per GPS5 message.
        let gps5 = devc_stream
            .find(&FourCC::GPS5)
            .and_then(|s| s.to_vec_f64())?;

        let mut lat_sum: f64 = 0.0;
        let mut lon_sum: f64 = 0.0;
        let mut alt_sum: f64 = 0.0;
        let mut sp2d_sum: f64 = 0.0;
        let mut sp3d_sum: f64 = 0.0;

        let len = gps5.len();

        gps5.iter().for_each(|v| {
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

        // Datetime for coordinate cluster
        // let gpsu: PrimitiveDateTime = devc_stream
        //     .find(&FourCC::GPSU)
        //         .and_then(|s| s.first_value())
        //         .and_then(|v| v.into())?;
        let gpsu: SystemTime = devc_stream
            .find(&FourCC::GPSU)
                .and_then(|s| s.first_value())
                .and_then(|v| v.into())?;
        // or better to return generic date than error?

        // GPS satellite lock level, 0 = no lock, 2 = 2D lock, 3 = 3D lock
        let gpsf: u32 = devc_stream
            .find(&FourCC::GPSF)
                .and_then(|s| s.first_value())
                .and_then(|v| v.into())?;

        // GPS precision
        let gpsp: u16 = devc_stream
            .find(&FourCC::GPSP)
                .and_then(|s| s.first_value())
                .and_then(|v| v.into())?;

        // let relative_time = devc_stream.time.as_ref()?.relative.to_owned();
        let relative_time = devc_stream.time.to_owned() // TODO impl Deref
            .map_or_else(|| Duration::default(), |t| t.relative.to_owned());

        // Create temporal 3d point with unix timestamp
        // let utc_nanos = gpsu.as_utc().unix_timestamp_nanos();
        // let systemtime = UNIX_EPOCH + Duration::nanoseconds_i128(utc_nanos);
        let point_xyzt = TemporalPoint3D::new(
            Point::new(
                lon_sum / len as f64 / lon_scl,
                lat_sum / len as f64 / lat_scl
            ),
            alt_sum / len as f64 / alt_scl,
            // systemtime
            gpsu
        );

        Some(Self {
            point: point_xyzt,
            speed2d: sp2d_sum / len as f64 / sp2d_scl,
            speed3d: sp3d_sum / len as f64 / sp3d_scl,
            // datetime: gpsu,
            dop: gpsp as f64 / 100.,
            fix: gpsf,
            timestamp: relative_time,
        })
    }

    /// For Hero11 and later models. These log at 10Hz.
    /// Returns the point cluster in the specified DEVC stream.
    /// GPS9 devices log datetime, GPS fix, GPS dop individually for each point.
    ///
    /// Stream name (`STNM`): "GPS (Lat., Long., Alt., 2D, 3D, days, secs, DOP, fix)"
    pub fn from_gps9(devc_stream: &Stream) -> Option<Vec<Self>> {
        // GPS9 contains more info than GPS5, including datetime per-point,
        // dilution of precision and satellite lock level:
        // [lat, lon, alt, 2d speed, 3d speed, days, seconds, dop, fix]
        // On average 10 coordinates per GPS9 message.
        // 230323 GPS9 is a Complex value (GPS5 is not). Added Into<Vec<f64>> for Value::Complex, works so far.
        let gps9 = devc_stream
            .find(&FourCC::GPS9)
            .and_then(|s| s.to_vec_f64())?;

        // REQUIRED
        let scale = devc_stream
            .find(&FourCC::SCAL)
            .and_then(|s| s.to_f64())?;

        // All points, no filtering on GPS fix/satellite lock
        // Relative timestamp not set
        // let mut raw_points: Vec<GoProPoint> = gps9.par_iter()
        let mut points: Vec<GoProPoint> = gps9.iter()
            .map(|gps| GoProPoint::from_gps9_slice(&gps, &scale))
            .collect();

        // let devc_time = devc_stream.time.as_ref()?.to_owned();
        // let t0 = devc_time.relative;
        // let dt0 = points.first()?.datetime;
        // points.iter_mut()
        //     .for_each(|p| p.time = t0 + (p.datetime - dt0));

        // Derive relative timestamp for video timeline.
        if let Some(ts) = devc_stream.time.as_ref() {
            let t0 = ts.relative;
            let dt0 = points.first()?.datetime().ok()?;
            for point in points.iter_mut() {
                point.timestamp = t0 + (point.datetime().ok()? - dt0);
            }
            // points.iter_mut()
            //     .for_each(|p| p.timestamp = t0 + (p.datetime().ok()? - dt0));
        }

        Some(points)
    }

    /// Returns datetime as string.
    pub fn datetime_to_string(&self) -> Result<String, GpmfError> {
        // this convesion only takes place due to how
        // time-rs handles systemtime, only via OffsetDateTime...
        // GoPro does not log timezone/offset.
        let odt = self.datetime()?;
        let pdt = PrimitiveDateTime::new(odt.date(), odt.time());
        primitivedatetime_to_string(&pdt)
    }
}
