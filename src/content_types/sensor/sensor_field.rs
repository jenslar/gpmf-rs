// use time::PrimitiveDateTime;

// use crate::Timestamp;

use std::fmt::Display;

use super::Orientation;

/// Generic sensor data struct for
/// - Accelerometer (acceleration, m/s2)
/// - Gyroscrope (rotation, rad/s)
/// - Gravity vector (direction of gravity)
#[derive(Debug, Default)]
pub struct SensorField {
    pub x: f64,
    pub y: f64,
    pub z: f64,
    // pub ext: Vec<SensorFieldExtension>
}

impl Display for SensorField {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "<x: {:>3.08}, y: {:>3.08}, z: {:>3.08}>", self.x, self.y, self.z)
    }
}

impl SensorField {
    pub fn new(
        xyz: &[f64],
        scale: f64,
        orientation: &Orientation,
    ) -> Option<Self> {
        let (x, y, z) = match orientation {
            Orientation::XYZ => (*xyz.get(0)?, *xyz.get(1)?, *xyz.get(2)?),
            Orientation::XZY => (*xyz.get(0)?, *xyz.get(2)?, *xyz.get(1)?),
            Orientation::YZX => (*xyz.get(2)?, *xyz.get(0)?, *xyz.get(1)?),
            Orientation::YXZ => (*xyz.get(1)?, *xyz.get(0)?, *xyz.get(2)?),
            Orientation::ZXY => (*xyz.get(1)?, *xyz.get(2)?, *xyz.get(0)?),
            Orientation::ZYX => (*xyz.get(2)?, *xyz.get(1)?, *xyz.get(0)?),
            Orientation::Invalid => return None
        };
        Some(Self{
            x: x/scale,
            y: y/scale,
            z: z/scale
        })
    }
}

// /// Used for GPS:
// /// - `GPS5` devices add GPS fix, dop, 2D speed, 3D speed (per-cluster)
// /// - `GPS9` devices add GPS fix, dop, 2D speed, 3D speed (per-point)
// pub enum SensorFieldExtension {
//     GPS {
//         /// 2D speed.
//         speed2d: f64,
//         /// 3D speed.
//         speed3d: f64,
//         // /// Heading 0-360 degrees
//         // heading: f64,
//         /// Datetime derived from `GPSU` message.
//         datetime: PrimitiveDateTime,
//         /// DOP, dilution of precision.
//         /// `GPSP` for `GPS5` device (Hero10 and earlier),
//         /// Value at index 7 in `GPS9` array (Hero11 and later)
//         /// A parsed value below 0.5 is good according
//         /// to GPMF docs.
//         dop: Option<f64>,
//         /// GPSF for GPS5 device (Hero10 and earlier),
//         /// Value nr 9 in GPS9 array (Hero11 and later)
//         fix: Option<u32>,
//         /// Timestamp
//         time: Option<Timestamp>,
//     },
//     GRAV,
// }