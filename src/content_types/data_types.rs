//! GPMF data type. These may change as new models are released,
//! and not all types are available in all cameras.

/// GPMF data type.
/// This will have to be updated if new data types are added or
/// future devices change the description given by the `STNM` stream.
/// 
/// Model names may be missing and will be updated with new sample data.
/// Only models that are confirmed for each data type are listed.
#[derive(Debug, Clone)]
pub enum DataType {
    /// `Accelerometer`.
    /// Present for Hero 7, 9.
    Accelerometer,
    /// `Accelerometer (up/down, right/left, forward/back)`.
    /// Present for Hero 5, 6.
    AccelerometerUrf,
    /// Present for Hero 9.
    AgcAudioLevel,
    /// Present for Hero 7.
    AverageLuminance,
    /// Present for Hero 9.
    CameraOrientation,
    /// `Exposure time (shutter speed)`.
    /// Present for Hero 5, 6, 7, 9.
    ExposureTime,
    /// `Face Coordinates and details`.
    /// Present for Hero 6, 7, 8, 9.
    FaceCoordinates,
    /// `GPS (Lat., Long., Alt., 2D speed, 3D speed)`.
    /// Present for Hero 5, 6, 7, 8, 9, 10, 11.
    Gps5,
    /// Present for Hero 11
    Gps9,
    /// `Gravity Vector`.
    /// Present for Hero 8, 9, 10, 11.
    GravityVector,
    /// `Gyroscope`. Present for Hero 7, 8, 9, 10, 11.
    Gyroscope,
    /// `Gyroscope (z,x,y)`. Present for Hero 5, 6
    GyroscopeZxy,
    /// `Image uniformity`.
    /// Present for Hero 7, 8, 9.
    ImageUniformity,
    /// `ImageOrientation`.
    /// Present for Hero 9.
    ImageOrientation,
    /// Present for Hero 9
    LrvFrameSkip,
    /// Present for Hero 9
    MicrophoneWet,
    /// Present for Hero 9
    MrvFrameSkip,
    /// Present for Hero 7
    PredominantHue,
    /// Present for Hero 7
    SceneClassification,
    /// Present for Fusion
    SensorGain,
    /// Present for Hero 7, 9
    SensorIso,
    /// Present for Hero 7
    SensorReadOutTime,
    /// Present for Hero 7, 9
    WhiteBalanceRgbGains,
    /// Present for Hero 7, 9
    WhiteBalanceTemperature,
    /// Present for Hero 9
    WindProcessing,
    Other(String),
}

impl DataType {
    /// Returns stream name (`STNM`) as specified inside GPMF.
    pub fn to_str(&self) -> &str {
        match self {
            // Confirmed for Hero 7, 8, 9, 11
            Self::Accelerometer => "Accelerometer",
            // Confirmed for Hero 5, 6
            Self::AccelerometerUrf => "Accelerometer (up/down, right/left, forward/back)",
            // Confirmed for Hero 8, 9 (' ,' typo exists in GPMF)
            Self::AgcAudioLevel => "AGC audio level[rms_level ,peak_level]",
            // Confirmed for Hero 7
            Self::AverageLuminance => "Average luminance",
            // Confirmed for Hero 9
            Self::CameraOrientation => "CameraOrientation",
            // Confirmed for Hero 7, 9, Fusion
            Self::ExposureTime => "Exposure time (shutter speed)",
            // Confirmed for Hero 7, 9
            Self::FaceCoordinates => "Face Coordinates and details",
            // Confirmed for Hero 5, 6, 7, 9, 10, Fusion
            Self::Gps5 => "GPS (Lat., Long., Alt., 2D speed, 3D speed)",
            // Confirmed for Hero 11
            Self::Gps9 => "GPS (Lat., Long., Alt., 2D, 3D, days, secs, DOP, fix)",
            // Confirmed for Hero 9
            Self::GravityVector => "Gravity Vector",
            // Confirmed for Hero 7, 9, 11.
            Self::Gyroscope => "Gyroscope",
            Self::GyroscopeZxy => "Gyroscope (z,x,y)",
            // Confirmed for Hero 7, 9
            Self::ImageUniformity => "Image uniformity",
            // Confirmed for Hero 9
            Self::ImageOrientation => "ImageOrientation",
            // Confirmed for Hero 9
            Self::LrvFrameSkip => "LRV Frame Skip",
            // Confirmed for Hero 9
            Self::MicrophoneWet => "Microphone Wet[mic_wet, all_mics, confidence]",
            // Confirmed for Hero 9
            Self::MrvFrameSkip => "MRV Frame Skip",
            // Confirmed for Hero 7
            Self::PredominantHue => "Predominant hue[[hue, weight], ...]",
            // Confirmed for Hero 7
            Self::SceneClassification => "Scene classification[[CLASSIFIER_FOUR_CC,prob], ...]",
            // Confirmed for Fusion
            Self::SensorGain => "Sensor gain",
            // Confirmed for Hero 7, 9
            Self::SensorIso => "Sensor ISO",
            // Confirmed for Hero 7
            Self::SensorReadOutTime => "Sensor read out time",
            // Confirmed for Hero 7, 9
            Self::WhiteBalanceRgbGains => "White Balance RGB gains",
            // Confirmed for Hero 7, 9
            Self::WhiteBalanceTemperature => "White Balance temperature (Kelvin)",
            // Confirmed for Hero 9
            Self::WindProcessing => "Wind Processing[wind_enable, meter_value(0 - 100)]",
            Self::Other(s) => s,
        }
    }

    /// Returns enum corresponding to stream name (`STNM`) specified in gpmf stream.
    /// If no results are returned despite the data being present,
    /// try using `Self::Other(String)` instead. Gpmf data can only be identified
    /// via its stream name free text description (`STNM`), which may differ between devices
    /// for the same kind of data.
    pub fn from_str(stream_type: &str) -> DataType {
        match stream_type {
            // Hero 7, 9 | Fusion
            "Accelerometer" => Self::Accelerometer,
            // Hero 5, 6
            "Accelerometer (up/down, right/left, forward/back)" => Self::AccelerometerUrf,
            // Hero 9 (comma spacing is correct)
            "AGC audio level[rms_level ,peak_level]" => Self::AgcAudioLevel,
            // Hero 7
            "Average luminance" => Self::AverageLuminance,
            // Hero 9
            "CameraOrientation" => Self::CameraOrientation,
            // Hero 7, 9, Fusion
            "Exposure time (shutter speed)" => Self::ExposureTime,
            // Hero 7, 9
            "Face Coordinates and details" => Self::FaceCoordinates,
            // Hero 7, 9
            "GPS (Lat., Long., Alt., 2D speed, 3D speed)" => Self::Gps5,
            "GPS (Lat., Long., Alt., 2D, 3D, days, secs, DOP, fix)" => Self::Gps9,
            // Hero 9
            "Gravity Vector" => Self::GravityVector,
            // Hero 7, 9 | Fusion
            "Gyroscope" => Self::Gyroscope,
            // Hero 5, 6
            "Gyroscope (z,x,y)" => Self::GyroscopeZxy,
            // Hero 7, 9
            "Image uniformity" => Self::ImageUniformity,
            // Hero 9
            "ImageOrientation" => Self::ImageOrientation,
            // Hero 9
            "LRV Frame Skip" => Self::LrvFrameSkip,
            // Hero 9
            "Microphone Wet[mic_wet, all_mics, confidence]" => Self::MicrophoneWet,
            // Hero 9
            "MRV Frame Skip" => Self::MrvFrameSkip,
            // Hero 7
            "Predominant hue[[hue, weight], ...]" => Self::PredominantHue,
            // Hero 7
            "Scene classification[[CLASSIFIER_FOUR_CC,prob], ...]" => Self::SceneClassification,
            // Fusion
            "Sensor gain (ISO x100)" => Self::SensorGain,
            // Hero 7, 9
            "Sensor ISO" => Self::SensorIso,
            // Hero 7
            "Sensor read out time" => Self::SensorReadOutTime,
            // Hero 7, 9
            "White Balance RGB gains" => Self::WhiteBalanceRgbGains,
            // Hero 7, 9
            "White Balance temperature (Kelvin)" => Self::WhiteBalanceTemperature,
            // Hero 9
            "Wind Processing[wind_enable, meter_value(0 - 100)]" => Self::WindProcessing,
            // Other
            s => Self::Other(s.to_owned()),
        }
    }
}
