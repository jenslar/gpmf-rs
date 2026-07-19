//! In-device sensor orientation.

/// Physical orientation of the sensor module
/// inside the camera,
/// i.e. the the way the data is
/// stored according to the right-hand
/// rule.
#[derive(Debug, Clone)]
pub enum ImuOrientation {
    XYZ,
    XZY,
    YZX,
    YXZ,
    ZXY,
    ZYX,
    Invalid
}

impl Default for ImuOrientation {
    fn default() -> Self {
        Self::Invalid
    }
}

impl From<&str> for ImuOrientation {
    fn from(value: &str) -> Self {
        match value.to_lowercase().as_str() {
            "xyz" => Self::XYZ,
            "xzy" => Self::XZY,
            "yzx" => Self::YZX,
            "yxz" => Self::YXZ,
            "zxy" => Self::ZXY,
            "zyx" => Self::ZYX,
            _ => Self::Invalid
        }
    }
}
