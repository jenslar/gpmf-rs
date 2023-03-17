use time::PrimitiveDateTime;

use crate::{
    FourCC,
    GpmfError,
    Stream,
    Timestamp
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
    /// the device will log latest known location with a
    /// GPS fix of `0`, meaning both time and location will be
    /// wrong.
    /// 
    /// Defaults to 2 if `gps_fix` is `None`.
    /// Uses the `GPSF` value.
    pub fn filter(&self, gps_fix: Option<u32>) -> Self {
        // GoPro has four levels: 0, 1, 2, 3
        let threshold = gps_fix.unwrap_or(2);
        let filtered = self.0.iter()
            .filter(|p| 
                match p.fix {
                    Some(f) => f >= threshold,
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
    /// GPSF
    pub fix: Option<u32>,
    /// GPSP
    pub precision: Option<u16>,
    /// Timestamp
    pub time: Option<Timestamp>,
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
            self.fix,
            self.precision,
            self.time,
        )
    }
}

/// Point derived from GPS STRM with STNM "GPS (Lat., Long., Alt., 2D speed, 3D speed)"
impl GoProPoint {
    /// Parse stream of type `STRM` with `STNM` "GPS (Lat., Long., Alt., 2D speed, 3D speed)",
    /// containing coordinate cluster into a single `Point` struct.
    /// Returns a linear average of values within a single GPS stream, lumped together
    /// once/second (only a single timestamp is logged for each cluster).
    /// GoPro GPS logs at 10 or 18Hz (depending on model) so on average 10 or 18 points are logged each second.
    /// For those who record while moving at very high velocities, a latitude dependent average could
    /// be implemented in a future release.
    pub fn new(devc_stream: &Stream) -> Option<Self> {
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

        // let mut gps5_count: usize = 0;

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
            fix: gpsf,
            precision: gpsp,
        })
    }

    pub fn datetime_to_string(&self) -> Result<String, GpmfError> {
        primitivedatetime_to_string(&self.datetime)
    }
}
