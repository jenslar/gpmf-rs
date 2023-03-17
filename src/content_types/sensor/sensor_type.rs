use std::fmt::Display;

use crate::{DataType, DeviceName};

#[derive(Debug, Clone, Copy)]
pub enum SensorType {
    Accelerometer,
    GravityVector,
    Gyroscope,
    Unknown
}

impl Default for SensorType {
    fn default() -> Self {
        Self::Unknown
    }
}

impl Display for SensorType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SensorType::Accelerometer => write!(f, "Accelerometer"),
            SensorType::GravityVector => write!(f, "GravityVector"),
            SensorType::Gyroscope => write!(f, "Gyroscope"),
            SensorType::Unknown => write!(f, "Unknown"),
        }
    }
}

impl SensorType {
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

    pub fn from_datatype(data_type: &DataType) -> Self {
        match &data_type {
            DataType::Accelerometer | DataType::AccelerometerUrf => Self::Accelerometer,
            DataType::GravityVector => Self::GravityVector,
            DataType::Gyroscope | DataType::GyroscopeZxy => Self::Gyroscope,
            _ => Self::Unknown
        }
    }
}