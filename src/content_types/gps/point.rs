use time::{Duration, OffsetDateTime, PlainDateTime, PrimitiveDateTime, ext::NumericalDuration, macros::datetime};

use crate::{FourCC, GpmfError, Stream, content_types::offsetdatetime_to_string};

/// Point derived from GPS data stream.
#[derive(Debug, Clone, PartialEq)]
pub struct GoProPoint {
    pub(crate) point: geo_types::Point<f64>,
    pub altitude: f64,
    /// 2D speed.
    pub speed2d: f64,
    /// 3D speed.
    pub speed3d: f64,
    /// Datetime.
    /// - GPS5 devices: derived from `GPSU` message, once per point cluster.
    /// - GPS9 devices: logged per point.
    pub datetime: OffsetDateTime,
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
            point: geo_types::Point::default(),
            altitude: f64::default(),
            speed2d: f64::default(),
            speed3d: f64::default(),
            datetime: Self::datetime_default(), // GoPro start date
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
        let gp = value.point;
        let mut wp = gpx::Waypoint::new(gp);

        wp.elevation = Some(value.altitude);
        wp.speed = Some(value.speed2d);
        wp.time = Some(value.datetime.into());

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
            self.point.y(),
            self.point.x(),
            self.altitude,
            self.speed2d,
            self.speed3d,
            self.dop,
            self.fix,
            self.timestamp,
        )
    }
}

impl GoProPoint {
    fn from_gps_slice(
        gps_slice: &[f64],   // GPS5 (all devices) or GPS9 (Hero11)
        scale_slice: &[f64], // SCAL (all devices)
    ) -> Self {
        assert_eq!(gps_slice.len(), scale_slice.len(),
            "Must be equal in size: GPS5/9 slice has length {}, but scale slice has length {}",
            gps_slice.len(),
            scale_slice.len()
        );
        assert!(gps_slice.len() < 10,
            "Size of GPMF GPS9 slice exceeds expected value"
        );

        // Default point with time set to GoPro basetime 2000-01-01
        let mut point = Self::default();
        // iterate over data and scale slices simultaneously,
        // i.e. data value will match correct scale value
        gps_slice.iter().zip(scale_slice)
            .enumerate()
            .for_each(|(i, (gps, scale))| {
                match i {
                    // 0 - 4 GPS5, GPS9
                    0 => point.set_latitude(gps/scale), // no direct access to x/y (lon/lat) fields
                    1 => point.set_longitude(gps/scale), // no direct access to x/y (lon/lat) fields
                    2 => point.altitude = gps/scale,
                    3 => point.speed2d = gps/scale,
                    4 => point.speed3d = gps/scale,

                    // 5 - 8 GPS9 only
                    5 => point.datetime += (gps/scale).days(),
                    6 => point.datetime += (gps/scale).seconds(),
                    7 => point.dop = gps/scale,
                    8 => point.fix = (gps/scale).round() as u32,
                    _ => (),
                }
            });

        point
    }

    pub fn point(&self) -> geo_types::Point<f64> {
        self.point
    }

    pub fn latitude(&self) -> f64 {
        self.point.y()
    }

    pub fn set_latitude(&mut self, latitude: f64) {
        self.point.set_y(latitude);
    }

    pub fn with_latitude(self, latitude: f64) -> Self {
        Self {
            point: geo_types::Point::new(self.point.x(), latitude),
            ..self
        }
    }

    pub fn longitude(&self) -> f64 {
        self.point.x()
    }

    pub fn set_longitude(&mut self, longitude: f64) {
        self.point.set_x(longitude);
    }

    pub fn with_longitude(self, longitude: f64) -> Self {
        Self {
            point: geo_types::Point::new(longitude, self.point.y()),
            ..self
        }
    }

    pub fn with_lat_lon(self, latitude: f64, longitude: f64) -> Self {
        Self {
            point: geo_types::Point::new(
                longitude,
                latitude
            ),
            ..self
        }
    }

    pub fn altitude(&self) -> f64 {
        self.altitude
    }

    pub fn speed2d(&self) -> f64 {
        self.speed2d
    }

    pub fn speed3d(&self) -> f64 {
        self.speed3d
    }

    pub fn dilution_of_precision(&self) -> f64 {
        self.dop
    }

    pub fn satellite_fix(&self) -> u32 {
        self.fix
    }

    pub fn timestamp(&self) -> Duration {
        self.timestamp
    }

    pub fn set_timestamp(&mut self, timestamp: Duration) {
        self.timestamp = timestamp;
    }

    pub fn increase_timestamp(&mut self, timestamp_delta: Duration) {
        self.timestamp += timestamp_delta;
    }

    pub fn with_timestamp(self, timestamp: Duration) -> Self {
        Self {
            timestamp,
            ..self
        }
    }

    pub fn datetime(&self) -> OffsetDateTime {
        self.datetime
    }

    pub fn datetime_as_string(&self) -> Result<String, GpmfError>  {
        offsetdatetime_to_string(&self.datetime)
    }

    fn datetime_default() -> OffsetDateTime {
        datetime!(2000-01-01 0:00 +0)
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
        let gps5_vecs = devc_stream
            .find(&FourCC::GPS5)
            .and_then(|s| s.to_vec_f64())?;

        let mut lat_sum: f64 = 0.0;
        let mut lon_sum: f64 = 0.0;
        let mut alt_sum: f64 = 0.0;
        let mut sp2d_sum: f64 = 0.0;
        let mut sp3d_sum: f64 = 0.0;

        let len = gps5_vecs.len();

        gps5_vecs.iter().for_each(|v| {
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

        // Datetime for coordinate cluster.
        let gpsu: OffsetDateTime = devc_stream
            .find(&FourCC::GPSU)
                .and_then(|s| s.first_value())
                .and_then(|v| v.into())?;

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
        let point_2d = geo_types::Point::new(
            lon_sum / len as f64 / lon_scl,
            lat_sum / len as f64 / lat_scl
        );

        Some(Self {
            point: point_2d,
            altitude: alt_sum / len as f64 / alt_scl,
            speed2d: sp2d_sum / len as f64 / sp2d_scl,
            speed3d: sp3d_sum / len as f64 / sp3d_scl,
            datetime: gpsu,
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
        let mut points: Vec<GoProPoint> = gps9.iter()
            .map(|gps| GoProPoint::from_gps_slice(&gps, &scale))
            .collect();

        // Derive relative timestamp for video timeline.
        if let Some(ts) = devc_stream.time.as_ref() {
            let t0 = ts.relative;
            let dt0 = points.first()?.datetime();
            points.iter_mut()
                .for_each(|point| point.timestamp = t0 + (point.datetime() - dt0));
        }

        Some(points)
    }
}
