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
