// use time::PrimitiveDateTime;

// use crate::Timestamp;

use std::fmt::Display;

use super::ImuOrientation;

/// Generic sensor data struct for
/// - Accelerometer (acceleration, m/s2)
/// - Gyroscrope (rotation, rad/s)
/// - Gravity vector (direction of gravity)
#[derive(Debug, Default, Clone, Copy)]
pub struct ImuSample {
    pub x: f64,
    pub y: f64,
    pub z: f64,
    // pub ext: Vec<SensorFieldExtension>
}

impl Display for ImuSample {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "<x: {:>3.08}, y: {:>3.08}, z: {:>3.08}>", self.x, self.y, self.z)
    }
}

impl ImuSample {
    pub fn new(
        xyz: &[f64],
        scale: f64,
        orientation: &ImuOrientation,
    ) -> Option<Self> {
        let (x, y, z) = match orientation {
            ImuOrientation::XYZ => (*xyz.get(0)?, *xyz.get(1)?, *xyz.get(2)?),
            ImuOrientation::XZY => (*xyz.get(0)?, *xyz.get(2)?, *xyz.get(1)?),
            ImuOrientation::YZX => (*xyz.get(2)?, *xyz.get(0)?, *xyz.get(1)?),
            ImuOrientation::YXZ => (*xyz.get(1)?, *xyz.get(0)?, *xyz.get(2)?),
            ImuOrientation::ZXY => (*xyz.get(1)?, *xyz.get(2)?, *xyz.get(0)?),
            ImuOrientation::ZYX => (*xyz.get(2)?, *xyz.get(1)?, *xyz.get(0)?),
            ImuOrientation::Invalid => return None
        };
        Some(Self{
            x: x/scale,
            y: y/scale,
            z: z/scale
        })
    }
}
