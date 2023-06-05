use crate::SensorType;

#[derive(Debug, Clone, Copy)]
pub enum SensorQuantifier {
    Acceleration,
    Rotation,
    GravityDirection,
    Unknown
}

impl Default for SensorQuantifier {
    fn default() -> Self {
        Self::Unknown
    }
}

impl std::fmt::Display for SensorQuantifier {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match &self {
            Self::Acceleration => write!(f, "Acceleration"),
            Self::Rotation => write!(f, "Rotation"),
            Self::GravityDirection => write!(f, "Gravity direction"),
            Self::Unknown => write!(f, "Unknown"),
        }
    }
}

impl From<&SensorType> for SensorQuantifier {
    fn from(value: &SensorType) -> Self {
        match &value {
            SensorType::Accelerometer => Self::Acceleration,
            SensorType::GravityVector => Self::GravityDirection,
            SensorType::Gyroscope => Self::Rotation,
            SensorType::Unknown => Self::Unknown,
        }
    }
}