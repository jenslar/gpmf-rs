use std::collections::HashSet;

use time::Duration;

use crate::{DataType, DeviceName, FourCC, Gpmf, GpmfError, ImuType, Stream};

use crate::{ImuSample, ImuOrientation, ImuQuantifier};

/// Sensor data from a single or multiple `DEVC` streams:
/// - Accelerometer, fields are acceleration (m/s2).
/// - Gyroscope, fields are rotation (rad/s).
/// - Gravity vector, fields are direction of gravity in relation to camera angle.
#[derive(Debug, Default, Clone)]
pub struct Imu {
    /// Camera device name
    pub device: DeviceName,
    /// Accelerometer, gyroscope, gravity vector
    pub sensor: ImuType,
    /// Units
    pub units: Option<String>,
    /// Physical quantity
    pub quantifier: ImuQuantifier,
    /// Total samples delivered so far
    pub total: u32,
    /// Sensor orientation
    pub orientation: ImuOrientation,
    pub samples: Vec<ImuSample>,
    /// Timestamp relative to video start.
    pub timestamp: Option<Duration>,
    /// Duration in video.
    pub duration: Option<Duration>,
}

impl Imu {
    /// Parse sensor data from single `DEVC` container (`Stream`).
    pub fn single(devc_stream: &Stream, sensor: &ImuType, device: &DeviceName) -> Option<Self> {
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

        let orientation = orientation_str
            .map(|s| ImuOrientation::from(s.as_str()))
            .unwrap_or(ImuOrientation::XZY);

        let units: Option<String> = devc_stream
            .find(&FourCC::SIUN)
            .and_then(|s| s.first_value())
            .and_then(|s| s.into());

        // let orientation = match orientation_str {
        //     Some(orin) => Orientation::from(orin.as_str()),
        //     // None => Orientation::ZXY
        //     // Changed to XZY: https://github.com/gopro/gpmf-parser/issues/170#issuecomment-1322414755
        //     None => Orientation::XZY
        // };

        let total: u32 = devc_stream
            .find(&FourCC::TSMP)
            .and_then(|s| s.first_value())
            .and_then(|s| s.into())?;

        // Set FourCC for raw data arrays
        let sensor_fourcc = match &sensor {
            ImuType::Accelerometer => FourCC::ACCL,
            ImuType::Gyroscope => FourCC::GYRO,
            ImuType::GravityVector => FourCC::GRAV,
            ImuType::Unknown => return None
        };

        let sensor_quantifier = ImuQuantifier::from(sensor);

        // Vec containing x, y, z values
        let sensor_samples = devc_stream.find(&sensor_fourcc)
            .and_then(|val| val.to_vec_f64())? // each contained vec should have exactly 3 values for 3D sensor data
            .iter()
            .filter_map(|xyz| ImuSample::new(&xyz, scale, &orientation))
            .collect::<Vec<_>>();

        Some(Self{
            device: device.to_owned(),
            sensor: sensor.to_owned(),
            units,
            quantifier:sensor_quantifier,
            total,
            orientation,
            samples: sensor_samples,
            timestamp: devc_stream.time_relative(),
            duration: devc_stream.time_duration()
        })
    }

    /// Returns compiled sensor data from GPMF.
    pub fn new(gpmf: &Gpmf, imu_type: &ImuType) -> Option<Self> {
        let mut device_name: HashSet<DeviceName> = gpmf.device_name()
            .iter()
            // .map(|n| DeviceName::from_str(n))
            .filter_map(|n| match DeviceName::from_str(n) {
                DeviceName::Unknown => None,
                name => Some(name)
            })
            .collect();

        // in case no or multiple devices are found
        // (older devices such as Hero5 should at least log "Camera" as device...)
        assert!(
            device_name.len() < 2,
            "Multiple devices named in GPMF data. Expected one device."
        );

        let imu = Self::default();
        if let Some(name) = device_name.drain().collect::<Vec<_>>().first() {
            let data_type = imu_type.as_datatype(name);

            let imu_data_streams = gpmf.filter(&data_type);

            let imus = imu_data_streams.iter()
                .filter_map(|stream| Self::single(stream, imu_type, name))
                .collect::<Vec<Self>>();

            return Some(imu.merge(&imus))
        }

        None
    }

    pub fn from_gpmf(gpmf: &Gpmf, sensor: &ImuType) -> Vec<Self> {
        let device_name: Vec<DeviceName> = gpmf.device_name()
            .iter()
            // .map(|n| DeviceName::from_str(n))
            .filter_map(|n| match DeviceName::from_str(n) {
                DeviceName::Unknown => None,
                name => Some(name)
            })
            .collect();
        // Get camera device name (listed first if GPMF from Karma drone)
        // to get data type (free text data identifier is model dependent)
        if let Some(name) = device_name.first() {
            let data_type = sensor.as_datatype(name);

            let sensor_data_streams = gpmf.filter(&data_type);

            return sensor_data_streams.iter()
                .filter_map(|stream| Self::single(stream, sensor, name))
                // .inspect(|s| println!("TS:  {:?}\nDUR: {:?}", s.timestamp, s.duration))
                .collect::<Vec<Self>>()
        }

        // Failure to determine device name returns empty vec
        Vec::new()
    }

    /// Merge sensor data.
    pub fn merge(&self, other: &[Self]) -> Self {
        let duration = self.duration.unwrap_or_default()
            + other.iter().map(|s| s.duration.unwrap_or_default()).sum::<Duration>();
        let mut samples: Vec<ImuSample> = self.samples.to_owned();
        let other_fields: Vec<ImuSample> = other.iter()
            .flat_map(|s| s.samples.to_owned())
            .collect();
        samples.extend(other_fields);

        Self {
            // device: self.device.to_owned(),
            // sensor: self.sensor.to_owned(),
            // units: self.units.to_owned(),
            // quantifier: self.qu,
            // total: todo!(),
            // orientation: todo!(),
            samples,
            // timestamp: todo!(),
            duration: Some(duration),
            ..self.to_owned()
        }
    }

    pub fn merge2(sensor: &[Self]) -> Option<Self> {
        if let Some(first) = sensor.first().cloned() {
            if sensor.len() == 1 {
                return Some(first);
            }

            // Timestamps
            let timestamp = first.timestamp;
            let duration = first.duration.unwrap_or_default()
                + sensor[1..].iter().map(|s| s.duration.unwrap_or_default()).sum::<Duration>();

            // Samples
            let samples: Vec<ImuSample> = sensor.iter()
                .flat_map(|s| s.samples.to_owned())
                .collect();

            return Some(Self {
                samples,
                timestamp,
                duration: Some(duration),
                ..first
            })
        };
        None
    }

    pub fn samples(&self) -> impl Iterator<Item = &ImuSample> {
        self.samples.iter()
    }

    // pub fn samples_t(&self) -> impl Iterator<Item = (&SensorSample, Option<Duration>)> {
    //     let sample_duration = self.sample_duration();
    //     self.samples.iter()
    //         .enumerate()
    //         .map(|(i, s)| {
    //             let t = self.timestamp
    //                 .map(|ts| ts + i as f64 * sample_duration);
    //             (s, t)
    //         })
    // }

    /// Returns sample rate.
    pub fn samplerate(&self) -> Option<f64> {
        if let Some(duration) = self.duration {
            return Some(self.samples.len() as f64 / duration.as_seconds_f64())
        }
        None
    }

    /// Returns average sample duration as fractional seconds.
    pub fn sample_duration(&self) -> Option<f64> {
        if let Some(duration) = self.duration {
            return Some(duration.as_seconds_f64() / self.samples.len() as f64)
        }
        None
    }

    /// Generate sample time offsets in seconds.
    pub fn sample_offsets(&self) -> Vec<f64> {
        if let Some(sample_duration) = self.sample_duration() && let Some(timestamp) = self.timestamp {
            // let durs = vec![sample_duration; self.samples.len()];
            // let sample_durations: Vec<f64> = durs.into_iter().scan(0., |state, sdur| state + sdur).collect();
            let timestamp_seconds = timestamp.as_seconds_f64();
            let sample_durations: Vec<f64> = (0..self.samples.len())
                .into_iter()
                .scan(0., |state, _i| {
                    *state = *state + sample_duration + timestamp_seconds;
                    Some(*state)
                }).collect();
            assert!(sample_durations.len() == self.samples.len());

            return sample_durations
        }
        Vec::new()
    }

    pub fn as_datatype(&self) -> DataType {
        self.sensor.as_datatype(&self.device)
    }

    /// Returns number of samples.
    pub fn len(&self) -> usize {
        self.samples.len()
    }

    /// Returns all x-axis values.
    pub fn x(&self) -> Vec<f64> {
        self.samples.iter().map(|f| f.x).collect()
    }

    /// Returns all y-axis values.
    pub fn y(&self) -> Vec<f64> {
        self.samples.iter().map(|f| f.y).collect()
    }

    /// Returns all z-axis values.
    pub fn z(&self) -> Vec<f64> {
        self.samples.iter().map(|f| f.z).collect()
    }

    /// Returns all x, y, z values as vector of tuples `(x, y, z)`.
    pub fn xyz(&self) -> Vec<(f64, f64, f64)> {
        self.samples.iter()
            .map(|f| (f.x, f.y, f.z))
            .collect()
    }

    /// Returns all x, y, z values with a timestamp as vector of tuples `(x, y, z, T)`.
    pub fn xyz_t(&self) -> Result<Vec<(f64, f64, f64, f64)>, GpmfError> {
        let timestamps = self.sample_offsets();
        println!("T {}", timestamps.len());
        println!("S {}", self.len());
        if timestamps.len() != self.len() {
            return Err(GpmfError::SamplesTimestampSizeMismatch)
        }
        Ok(self.samples
            .iter()
            .zip(timestamps)
            .map(|(sample, time)| (sample.x, sample.y, sample.z, time))
            .collect())
    }

    /// Linear mean value of all x values.
    pub fn x_mean(&self) -> f64 {
        mean_value(&self.x())
    }

    /// Linear mean value of all x values.
    pub fn y_mean(&self) -> f64 {
        mean_value(&self.y())
    }

    /// Linear mean value of all x values.
    pub fn z_mean(&self) -> f64 {
        mean_value(&self.z())
    }

    /// Returns linear mean values of all x, y, z values as tuple `(x, y, z)`.
    pub fn xyz_mean(&self) -> (f64, f64, f64) {
        let (x, y, z) = self.samples.iter()
            .fold((0., 0., 0.), |acc, f| (acc.0 + f.x, acc.1 + f.y, acc.2 + f.z));
        let len = self.samples.len() as f64;

        (x / len, y / len, z / len)
    }

    /// Returns the magnitude for each sample as a scalar (vector length).
    /// If `sample_window_size` is set, the magnitude scalar value
    /// will be derived using averages of those windows.
    pub fn magnitude(&self, sample_window_size: Option<usize>) -> Vec<f64> {
        if let Some(window_size) = sample_window_size {
            // let mut sample_start: usize = 0;
            // let mut magnitudes: Vec<f64> = Vec::new();
            // let mut sample_average: Vec<f64> = Vec::new();
            // for (i, (x, y, z)) in self.xyz().iter().enumerate() {
            //     if i - sample_start > window_size {
            //         // calculate average
            //         magnitudes.push(sample_average.iter().sum::<f64>() / sample_average.len() as f64);
            //         // empty sample average
            //         sample_average.clear();
            //         // update sample start last
            //         sample_start = i;
            //     }
            //     // push samples here
            //     sample_average.push((x.powi(2) + y.powi(2) + z.powi(2)).sqrt());
            // }

            // magnitudes
            let mut sample_start: usize = 0;
            let mut magnitudes: Vec<f64> = Vec::new();
            let mut sample_average_x: Vec<f64> = Vec::new();
            let mut sample_average_y: Vec<f64> = Vec::new();
            let mut sample_average_z: Vec<f64> = Vec::new();
            let len = self.len();
            for (i, (x, y, z)) in self.xyz().iter().enumerate() {
                if i - sample_start > window_size || i + 1 == len {
                    // calculate average
                    let avg_x = sample_average_x.iter().sum::<f64>() / sample_average_x.len() as f64;
                    let avg_y = sample_average_y.iter().sum::<f64>() / sample_average_y.len() as f64;
                    let avg_z = sample_average_z.iter().sum::<f64>() / sample_average_z.len() as f64;

                    // add magnitude from average value
                    magnitudes.push((avg_x.powi(2) + avg_y.powi(2) + avg_z.powi(2)).sqrt());

                    // empty sample averages
                    sample_average_x.clear();
                    sample_average_y.clear();
                    sample_average_z.clear();

                    // update sample start last
                    sample_start = i;
                }

                // push samples here
                sample_average_x.push(*x);
                sample_average_y.push(*y);
                sample_average_z.push(*z);
            }

            magnitudes
        } else {
            self
                .xyz()
                .iter()
                .map(|(x, y, z)| (x.powi(2) + y.powi(2) + z.powi(2)).sqrt())
                .collect()
        }
    }

    pub fn magnitude_t(&self, sample_window_seconds: Option<f64>) -> Result<Vec<(f64, f64)>, GpmfError>{
        if let Some(timespan) = sample_window_seconds {
            // let mut sample_start_seconds: f64 = 0.;
            // let mut magnitudes_t: Vec<(f64, f64)> = Vec::new();
            // let mut sample_average: Vec<f64> = Vec::new();
            // for (i, (x, y, z, t)) in self.xyz_t()?.iter().enumerate() {
            //     if t - sample_start_seconds > timespan {
            //         // calculate dot average

            //         magnitudes_t.push((sample_average.iter().sum::<f64>() / sample_average.len() as f64, sample_start_seconds));
            //         // empty sample average
            //         // println!("{:6} new sample, LEN: {} T: {t}", i+1, sample_average.len());
            //         sample_average.clear();
            //         // update sample start last
            //         sample_start_seconds = *t;
            //     }
            //     // push samples here
            //     sample_average.push((x.powi(2) + y.powi(2) + z.powi(2)).sqrt());
            // }
            let mut sample_start_seconds: f64 = 0.;
            let mut magnitudes_t: Vec<(f64, f64)> = Vec::new();
            let mut sample_average_x: Vec<f64> = Vec::new();
            let mut sample_average_y: Vec<f64> = Vec::new();
            let mut sample_average_z: Vec<f64> = Vec::new();
            let xyz_t = self.xyz_t()?;
            let end_t = xyz_t.last().map(|(.., t)| t).ok_or_else(|| GpmfError::NoData)?;
            for (x, y, z, t) in xyz_t.iter() {
                if t - sample_start_seconds > timespan || t == end_t {
                    // calculate average
                    let avg_x = sample_average_x.iter().sum::<f64>() / sample_average_x.len() as f64;
                    let avg_y = sample_average_y.iter().sum::<f64>() / sample_average_y.len() as f64;
                    let avg_z = sample_average_z.iter().sum::<f64>() / sample_average_z.len() as f64;

                    // add magnitude from average value
                    magnitudes_t.push(((avg_x.powi(2) + avg_y.powi(2) + avg_z.powi(2)).sqrt(), *t));

                    // empty sample averages
                    sample_average_x.clear();
                    sample_average_y.clear();
                    sample_average_z.clear();

                    // update sample start time last
                    sample_start_seconds = *t;
                }

                // push samples here
                sample_average_x.push(*x);
                sample_average_y.push(*y);
                sample_average_z.push(*z);
            }

            Ok(magnitudes_t)
        } else {
            Ok(self
                .xyz_t()?
                .iter()
                .map(|(x, y, z, t)| ((x.powi(2) + y.powi(2) + z.powi(2)).sqrt(), *t))
                .collect())
        }
    }

    // pub fn downsample_time(&self, downsample_window_seconds: f64) {}

    /// Linear downsample, value based. I.e. each chunk of
    /// `downsample_factor` number of points will be downsampled
    /// to a single value.
    pub fn downsample(&self, downsample_factor: usize) -> Self {
        if downsample_factor == 0 || downsample_factor == 1 {
            return self.to_owned()
        }

        // Number of samples
        let len = self.len();

        // Get number of chunks with same size as downsample factor,
        // which will be downsampled to one point per chunk.
        // Then get the summed remainder values to be downsampled
        // by the numer of values remaining.

        // Iterator over chunks with size ädownsample_factor'
        let vec_x = self.x(); // need to bind or temp value is dropped
        let chunks_x = vec_x.chunks_exact(downsample_factor as usize);
        // Summed remainder values.
        let rem_x = chunks_x.remainder().iter().sum::<f64>();

        let vec_y = self.y(); // need to bind or temp value is dropped
        let chunks_y = vec_y.chunks_exact(downsample_factor as usize);
        // Summed remainder values.
        let rem_y = chunks_y.remainder().iter().sum::<f64>();

        let vec_z = self.z(); // need to bind or temp value is dropped
        let chunks_z = vec_z.chunks_exact(downsample_factor as usize);
        // Summed remainder values.
        let rem_z = chunks_z.remainder().iter().sum::<f64>();
        // let iter_y = self.y().chunks_exact(downsample_factor as usize);
        // let rem_y = iter_y.remainder().iter().sum::<f64>();
        // let iter_z = self.z().chunks_exact(downsample_factor as usize);
        // let rem_z = iter_z.remainder().iter().sum::<f64>();
        let (mut x, mut y, mut z) = chunks_x.zip(chunks_y).zip(chunks_z)
            .map(|((x, y), z)| (
                x.iter().sum::<f64>() / downsample_factor as f64,
                y.iter().sum::<f64>() / downsample_factor as f64,
                z.iter().sum::<f64>() / downsample_factor as f64
            ))
            .collect::<(Vec<f64>, Vec<f64>, Vec<f64>)>();
        // Number of values summed remainder corresponds to
        let rem_len = len % downsample_factor as usize;
        // Downsample by number of remaining values
        if rem_len > 0 {
            x.push(rem_x / rem_len as f64);
            y.push(rem_y / rem_len as f64);
            z.push(rem_z / rem_len as f64);
        }
        // Create new samples
        let samples = x.into_iter().zip(y).zip(z)
            .map(|((x, y), z)| ImuSample {x, y, z})
            .collect();
        return Self {
            samples,
            ..self.to_owned()
        }
    }
}

/// Returns the linear mean value.
fn mean_value(values: &[f64]) -> f64 {
    values.iter().sum::<f64>() / values.len() as f64
}

/// Returns the median value.
fn median_value(values: &[f64]) {

}
