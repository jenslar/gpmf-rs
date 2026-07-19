use std::fmt::Display;

use crate::{DataType, DeviceName};

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ImuType {
    Accelerometer,
    // AccelerometerMagnitude,
    GravityVector,
    // GravityVectorMagnitude,
    Gyroscope,
    // GyroscopeMagnitude,
    Unknown
}

impl Default for ImuType {
    fn default() -> Self {
        Self::Unknown
    }
}

impl Display for ImuType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ImuType::Accelerometer => write!(f, "Accelerometer"),
            // ImuType::AccelerometerMagnitude => write!(f, "Accelerometer, magnitude"),
            ImuType::GravityVector => write!(f, "Gravity Vector"),
            // ImuType::GravityVectorMagnitude => write!(f, "Gravity Vector, magnitude"),
            ImuType::Gyroscope => write!(f, "Gyroscope"),
            // ImuType::GyroscopeMagnitude => write!(f, "Gyroscope, magnitude"),
            ImuType::Unknown => write!(f, "Unknown"),
        }
    }
}

impl From<&str> for ImuType {
    fn from(value: &str) -> Self {
        match value.to_lowercase().as_str() {
            "acc" | "accl" | "accelerometer" => Self::Accelerometer,
            "grv" | "grav" | "gravityvector" | "gravity vector" => Self::GravityVector,
            "gyr" | "gyro" | "gyroscope" => Self::Gyroscope,
            _ => Self::Unknown
        }
    }
}

impl ImuType {
    /// Convert `SensorType` to `DataType`
    pub fn as_datatype(&self, device: &DeviceName) -> DataType {
        match &self {
            Self::Accelerometer => match device {
                DeviceName::Hero5Black | DeviceName::Hero6Black => DataType::AccelerometerUrf,
                _ => DataType::Accelerometer
            }
            Self::GravityVector => DataType::GravityVector,
            Self::Gyroscope => match device {
                DeviceName::Hero5Black | DeviceName::Hero6Black => DataType::GyroscopeZxy,
                _ => DataType::Gyroscope
            },
            Self::Unknown => DataType::Other("Unkown".to_owned())
        }
    }

    /// Convert `DataType` to `SensorType`
    pub fn from_datatype(data_type: &DataType) -> Self {
        match &data_type {
            DataType::Accelerometer | DataType::AccelerometerUrf => Self::Accelerometer,
            DataType::GravityVector => Self::GravityVector,
            DataType::Gyroscope | DataType::GyroscopeZxy => Self::Gyroscope,
            _ => Self::Unknown
        }
    }

    pub fn units(&self) -> &str {
        match &self {
            Self::Accelerometer => "m/s²",
            Self::GravityVector => "N/A",
            Self::Gyroscope => "rad/s",
            Self::Unknown => "N/A",
        }
    }

    pub fn quantifier(&self) -> &str {
        match &self {
            Self::Accelerometer => "Acceleration",
            Self::GravityVector => "N/A",
            Self::Gyroscope => "Rotation",
            Self::Unknown => "N/A",
        }
    }
}
