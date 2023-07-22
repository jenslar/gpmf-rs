//! Covers GoPro 3D sensors. Supported sensor data:
//! - Accelerometer
//! - Gyroscope
//! - Gravity Vector

mod sensor_data;
mod sensor_type;
mod sensor_field;
mod sensor_quantifier;
mod orientation;

// pub use accl::{Acceleration, Accelerometer};
// pub use gyro::{Rotation, Gyroscope};
pub use sensor_data::SensorData;
pub use sensor_type::SensorType;
pub use sensor_field::SensorField;
pub use sensor_quantifier::SensorQuantifier;
pub use orientation::Orientation;