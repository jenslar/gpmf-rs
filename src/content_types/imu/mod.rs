//! Covers GoPro 3D sensors. Supported sensor data:
//! - Accelerometer
//! - Gyroscope
//! - Gravity Vector

mod imu;
mod imu_type;
mod sample;
mod quantifier;
mod orientation;

// pub use accl::{Acceleration, Accelerometer};
// pub use gyro::{Rotation, Gyroscope};
pub use imu::Imu;
pub use imu_type::ImuType;
pub use sample::ImuSample;
pub use quantifier::ImuQuantifier;
pub use orientation::ImuOrientation;
