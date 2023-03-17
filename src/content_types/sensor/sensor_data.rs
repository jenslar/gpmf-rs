use time::Duration;

use crate::{DeviceName, DataType, FourCC, Stream, Gpmf, SensorType};

use super::{SensorField, Orientation, SensorQuantifier};

/// Sensor data from a single `DEVC` stream:
/// - Accelerometer, fields are acceleration (m/s2)
/// - Gyroscope, fields are rotation (rad/s)
/// - Gravity vector, fields are direction of gravity in relation to the camera
#[derive(Debug, Default)]
pub struct SensorData {
    /// Camera device name
    pub device: DeviceName,
    /// Accelerometer, gyroscope, gravimeter
    pub sensor: SensorType,
    /// Units
    pub units: Option<String>,
    /// Physical quantity
    pub quantifier: SensorQuantifier,
    /// Sensor orientation
    pub orientation: Orientation,
    pub fields: Vec<SensorField>,
    /// Timestamp relative to video start.
    pub timestamp: Option<Duration>,
    /// Duration in video.
    pub duration: Option<Duration>,
}

impl SensorData {
    pub fn new(devc_stream: &Stream, sensor: &SensorType, device: &DeviceName) -> Option<Self> {
        // Scale, should only be a single value for Gyro
        let scale = *devc_stream
            .find(&FourCC::SCAL)
            .and_then(|s| s.to_f64())?
            .first()?;

        // See https://github.com/gopro/gpmf-parser/issues/165#issuecomment-1207241564
        let orientation_str: Option<String> = devc_stream
            .find(&FourCC::ORIN)
            .and_then(|s| s.first_value())
            .and_then(|s| s.into());

        let units: Option<String> = devc_stream
            .find(&FourCC::SIUN)
            .and_then(|s| s.first_value())
            .and_then(|s| s.into());
        
        let orientation = match orientation_str {
            Some(orin) => Orientation::from(orin.as_str()),
            None => Orientation::ZXY
        };

        // Set FourCC for raw data arrays
        let sensor_fourcc = match &sensor {
            SensorType::Accelerometer => FourCC::ACCL,
            SensorType::Gyroscope => FourCC::GYRO,
            SensorType::GravityVector => FourCC::GRAV,
            SensorType::Unknown => return None
        };

        let sensor_quantifier = SensorQuantifier::from(sensor);

        // Vec containing rotation x, y, z values,
        // but order needs to be checked
        let sensor_fields = devc_stream.find(&sensor_fourcc)
            .and_then(|val| val.to_vec_f64())?
            .iter()
            // .filter_map(|xyz| SensorField::new(&xyz, scale, &orientation, &sensor_field_type))
            .filter_map(|xyz| SensorField::new(&xyz, scale, &orientation))
            .collect::<Vec<_>>();

        Some(Self{
            device: device.to_owned(),
            sensor: sensor.to_owned(),
            units,
            quantifier:sensor_quantifier,
            orientation,
            fields: sensor_fields,
            timestamp: devc_stream.time_relative(),
            duration: devc_stream.time_duration()
        })
    }

    pub fn from_gpmf(gpmf: &Gpmf, sensor: &SensorType) -> Vec<Self> {
        let device_name: Vec<DeviceName> = gpmf.device_name()
            .iter()
            // .map(|n| DeviceName::from_str(n))
            .filter_map(|n| DeviceName::from_str(n))
            .collect();
        // Get camera device name (listed first if GPMF from Karma drone)
        // to get data type (free text data identifier is model dependent)
        if let Some(device) = device_name.first() {
            let data_type = sensor.as_datatype(device);

            let sensor_data = gpmf.filter(&data_type);

            return sensor_data.iter()
                .filter_map(|stream| Self::new(stream, sensor, device))
                .collect::<Vec<Self>>()
        }

        // Failure to determine device name returns empty vec
        Vec::new()
    }

    pub fn as_datatype(&self) -> DataType {
        self.sensor.as_datatype(&self.device)
    }

    pub fn len(&self) -> usize {
        self.fields.len()
    }
}
