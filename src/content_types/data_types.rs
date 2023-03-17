/// Stream type, mostly for internal use.
/// This will have to be updated if new stream types are added or STNM free text descriptions change.
#[derive(Debug, Clone)]
pub enum DataType {
    /// Accelerometer
    Accelerometer,           // Hero 7, 9
    /// Accelerometer (up/down, right/left, forward/back)
    AccelerometerUrf,        // Hero 5, 6
    AgcAudioLevel,           // Hero 9
    AverageLuminance,        // Hero 7
    CameraOrientation,       // Hero 9
    ExposureTime,            // Hero 7, 9
    FaceCoordinates,         // Hero 7, 9
    Gps5,                    // Hero 5, 6, 7, 9, 10, 11
    Gps9,                    // Hero 11
    GravityVector,           // Hero 9
    Gyroscope,               // Hero 7, 9
    GyroscopeZxy,            // Hero 5, 6
    ImageUniformity,         // Hero 7, 9
    ImageOrientation,        // Hero 9
    LrvFrameSkip,            // Hero 9
    MicrophoneWet,           // Hero 9
    MrvFrameSkip,            // Hero 9
    PredominantHue,          // Hero 7
    SceneClassification,     // Hero 7
    SensorGain,              // Fusion
    SensorIso,               // Hero 7, 9
    SensorReadOutTime,       // Hero 7
    WhiteBalanceRgbGains,    // Hero 7, 9
    WhiteBalanceTemperature, // Hero 7, 9
    WindProcessing,          // Hero 9
    Other(String),
}

impl DataType {
    /// Returns stream name (`STNM`) specified in gpmf documentation as a string slice.
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
