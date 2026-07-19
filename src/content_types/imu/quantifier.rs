use crate::ImuType;

#[derive(Debug, Clone, Copy)]
pub enum ImuQuantifier {
    Acceleration,
    Rotation,
    GravityDirection,
    Unknown
}

impl Default for ImuQuantifier {
    fn default() -> Self {
        Self::Unknown
    }
}

impl std::fmt::Display for ImuQuantifier {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match &self {
            Self::Acceleration => write!(f, "Acceleration"),
            Self::Rotation => write!(f, "Rotation"),
            Self::GravityDirection => write!(f, "Gravity direction"),
            Self::Unknown => write!(f, "Unknown"),
        }
    }
}

impl From<&ImuType> for ImuQuantifier {
    fn from(value: &ImuType) -> Self {
        match &value {
            ImuType::Accelerometer => Self::Acceleration,
            ImuType::GravityVector => Self::GravityDirection,
            ImuType::Gyroscope => Self::Rotation,
            ImuType::Unknown => Self::Unknown,
        }
    }
}
