//! GPMF Four CC, i.e. general stream identifier.
//! Not all are covered or documented, hence `FourCC::Other(String)`.
//! `FourCC::Invalid` is there to check for zero padding in MP4 `udta` atom GPMF streams,
//! which will otherwise erronously be parsed as valid GPMF FourCC.

use std::io::{Cursor, Read};

use crate::GpmfError;

/// FourCC enum. Descriptions lifted from official GPMF documentation (<https://github.com/gopro/gpmf-parser>)
#[derive(Debug, Clone, PartialEq)]
pub enum FourCC {
    // FOURCC RESERVED FOR GPMF STRUCTURE
    /// unique device source for metadata
    DEVC,
    /// device/track ID
    /// Auto generated unique-ID for managing a large number of connect devices, camera, karma and external BLE devices
    DVID,
    /// device name
    /// Display name of the device like "Karma 1.0", this is for communicating to the user the data recorded, so it should be informative.
    DVNM,
    /// Nested signal stream of metadata/telemetry
    /// Metadata streams are each nested with STRM
    STRM,
    /// Stream name
    /// Display name for a stream like "GPS RAW", this is for communicating to the user the data recorded, so it should be informative.
    STNM,
    /// Comments for any stream
    /// Add more human readable information about the stream
    RMRK,
    /// Scaling factor (divisor)
    /// Sensor data often needs to be scaled to be presented with the correct units. SCAL is a divisor.
    SCAL,
    /// Standard Units (like SI)
    /// If the data can be formatted in GPMF's standard units, this is best. E.g. acceleration as "m/s²". SIUN allows for simple format conversions.
    SIUN,
    /// Display units
    /// While SIUN is preferred, not everything communicates well via standard units. E.g. engine speed as "RPM" is more user friendly than "rad/s".
    UNIT,
    /// Typedefs for complex structures Not everything has a simple repeating type. For complex structure TYPE is used to describe the data packed within each sample.
    TYPE,
    /// Total Samples delivered Internal field that counts all the sample delivered since record start, and is automatically computed.
    TSMP,
    /// Time Offset Rare. An internal field that indicates the data is delayed by 'x' seconds.
    TIMO,
    /// Empty payload count
    EMPT,

    // DEVICE/DATA SPECIFIC FOURCC
    /// HERO8Black  Audio Levels    10Hz    dBFS    RMS and peak audio levels in dBFS
    AALP,
    /// Fusion  3-axis accelerometer    200 m/s²    Data order -Y,X,Z
    /// HERO5BlackAndSession    3-axis accelerometer    200 m/s²    Data order Z,X,Y
    /// HERO6Black  3-axis accelerometer    200 m/s²    Data order Y,-X,Z
    ACCL,
    /// HERO6Black  Auto Low Light frame Duration   24, 25 or 30 (based video frame rate)   n/a ALL extended exposure   time
    ALLD,
    /// GoProMAX    Camera ORIentation  frame rate  n/a Quaternions for the camera orientation since capture start
    /// HERO8Black  Camera ORIentation  frame rate  n/a Quaternions for the camera orientation since capture start
    CORI,
    /// GoProMAX    Disparity track (360 modes) frame rate  n/a 1-D depth map for the objects seen by the two lenses
    DISP,
    /// HERO6Black  Face detection boundaring boxes 12, 12.5 or 15 (based video frame rate) n/a struct ID,x,y,w,h -- not    supported in HEVC modes
    /// HERO7Black  Face boxes and smile confidence at base frame rate 24/25/30 n/a struct ID,x,y,w,h,unused[17],smile
    FACE,
    /// HERO6Black  Faces counted per frame 12, 12.5 or 15 (based video frame rate) n/a Not supported in HEVC modes
    /// HERO7Black  removed n/a n/a
    FCNM,
    /// HERO5Black+  latitude, longitude, altitude (WGS 84), 2D ground speed, and 3D speed   18  deg, deg, m, m/s, m/s   
    GPS5,
    /// HERO5Black+  GPS Fix 1   n/a Within the GPS stream: 0 - no lock, 2 or 3 - 2D or 3D Lock
    GPSF,
    /// HERO5Black+  GPS Precision - Dilution of Precision (DOP x100)    1   n/a Within the GPS stream, under 500 is     good. For more information of GPSP, (or DOP) see https://en.wikipedia.org/wiki/Dilution_of_precision_(navigation)
    GPSP,
    /// HERO5Black  UTC time and data from GPS  1   n/a Within the GPS stream
    GPSU,
    /// Hero 8(?), 9 GPS Altitude, added in v1.50, before used WGS 84 for alt above the ellipsoid
    GPSA,
    /// GoProMAX    GRAvity Vector  frame rate  n/a Vector for the direction for gravity
    /// HERO8Black  GRAvity Vector  frame rate  n/a Vector for the direction for gravity
    GRAV,
    /// Fusion  3-axis gyroscope    3200    rad/s   Data order -Y,X,Z
    /// HERO5BlackAndSession    3-axis gyroscope    400 rad/s   Data order Z,X,Y
    /// HERO6Black  3-axis gyroscope    200 rad/s   Data order Y,-X,Z
    GYRO,
    HUES, // HERO7Black  Predominant hues over the frame 8 - 10  n/a struct ubyte hue, ubyte weight, HSV_Hue = hue x 360/255
    // GoProMAX    Image ORIentation   frame rate  n/a Quaternions for the image orientation relative to the camera body
    // HERO8Black  Image ORIentation   frame rate  n/a Quaternions for the image orientation relative to the camera body
    IORI,
    /// HERO6Black  Sensor ISO  24, 25 or 30 (based video frame rate)   n/a replaces ISOG, has the same function
    ISOE,
    /// Fusion  Image sensor gain   increased to 60 n/a per frame exposure metadata
    /// HERO5BlackAndSession    Image sensor gain   24, 25 or 30 (based video frame rate)   n/a HERO5 v2 or greater     firmware
    ISOG,
    /// HERO9Black  Low res video frame SKiP    frame rate  n/a GoPro internal usage. Same as MSKP for the LRV video    file (when present.) This improves sync with the main video when using the LRV as a proxy.
    LSKP,
    /// Fusion  magnetometer    24  µT  Camera pointing direction
    /// GoProMAX    MAGNnetometer   24  µT  Camera pointing direction x,y,z (valid in v2.0 firmware.)
    MAGN,
    /// HERO9Black  Main video frame SKiP   frame rate  n/a GoPro internal usage. Number frames skips or duplicated from sensor image captured to encoded frame. Normally 0. This is used for visual effects when precision timing of the   video frame is required.
    MSKP,
    /// HERO8Black  Microphone is WET   10Hz    n/a marks whether some of the microphones are wet
    MWET,
    /// HERO7Black  Scene classifier in probabilities   8 - 10  n/a FourCC scenes: SNOW, URBAn, INDOor, WATR, VEGEtation,    BEACh
    /// Hero 6a (not 6), 7, 8, 9 Orientation, accelerometer
    ORIN,
    /// Hero 6a (not 6), 7, 8 Orientation, accelerometer
    ORIO,
    /// Hero 6a (not 6), 7, 8 Orientation, accelerometer
    MTRX,
    /// Scene?
    SCEN,
    /// Fusion  Exposure time   increased to 60 s   per frame exposure metadata
    /// HERO5BlackAndSession    Exposure time   24, 25 or 30 (based video frame rate)   s   HERO5 v2 or greater firmware
    SHUT,
    /// HERO7Black  Sensor Read Out Time    at base frame rate 24/25/30 n/a this moves to a global value in HERO8
    SROT,
    /// Fusion and later (?)  microsecond timestamps  1   µs  Increased precision for post stablization
    STMP,
    /// HERO7Black  Image uniformity    8 - 10  range 0 to 1.0 where 1.0 is a solid color   
    UNIF,
    /// HERO6Black  White Balance in Kelvin 24, 25 or 30 (based video frame rate)   n/a Classic white balance info
    WBAL,
    /// HERO8Black  Wind Processing 10Hz    n/a marks whether wind processing is active
    WNDM,
    /// HERO6Black  White Balance RGB gains 24, 25 or 30 (based video frame rate)   n/a Geeky white balance info
    WRGB,
    /// HERO7Black  Luma (Y) Average over the frame 8 - 10  n/a range 0 (black) to 255 (white)
    YAVG,

    // Content FourCC
    /// In GPSA (GPS Altitude) for GPS stream: Mean Sea Level
    MSLV,
    /// HERO7Black Scene classification Snow
    SNOW,
    /// HERO7Black Scene classification Urban
    URBA,
    /// HERO7Black Scene classification Indoors
    INDO,
    /// HERO7Black Scene classification Water
    WATR,
    /// HERO7Black Scene classification Vegetation
    VEGE,
    /// HERO7Black Scene classification Beach
    BEAC,

    // MP4 user data atom (`udta`) only
    /// MP4 `udta` firmware version
    FIRM,
    /// MP4 `udta` lens serial number (unconfirmed)
    LENS,
    /// MP4 `udta` camera (?)
    CAME,
    /// MP4 `udta` settings (?)
    SETT,
    /// MP4 `udta` unknown
    AMBA,
    /// MP4 `udta` unknown
    MUID,
    /// MP4 `udta` unknown
    HMMT,
    /// MP4 `udta` unknown
    BCID,
    /// MP4 `udta` unknown
    GUMI,

    // JPEG GPMF FourCC
    MINF,

    /// Mainly for checking and invalidating 0-padding
    /// in MP4 `udta` GPMF data.
    Invalid,
    
    /// Undocumented FourCC, such as for those found in GoPro MP4 `udta` atom's GPMF section
    Other(String),
}

impl Default for FourCC {
    fn default() -> Self {
        FourCC::Invalid
    }
}

impl FourCC {
    pub fn new(cursor: &mut Cursor<Vec<u8>>) -> Result<Self, GpmfError> {
    // pub fn new(cursor: &mut Cursor<&[u8]>) -> Result<Self, GpmfError> {
        let mut buf = vec![0_u8; 4];
        let _len = cursor.read(&mut buf)?;
        // if len != buf.len() {
        //     return Err(GpmfError::ReadMismatch{got: len as u64, expected: buf.len() as u64})
        // }

        Self::from_slice(&buf)
    }
    pub fn new2(cursor: &[u8]) -> Result<Self, GpmfError> {
    // pub fn new(cursor: &mut Cursor<&[u8]>) -> Result<Self, GpmfError> {
        // let mut buf = vec![0_u8; 4];
        // let len = cursor.read(&mut buf)?;
        // if len != buf.len() {
        //     return Err(GpmfError::ReadMismatch{got: len as u64, expected: buf.len() as u64})
        // }

        Self::from_slice(cursor)
    }

    /// Generate FourCC enum from `&str`.
    fn from_slice(slice: &[u8]) -> Result<Self, GpmfError> {
        // assert_eq!(
        //     slice.len(),
        //     4,
        //     "FourCC must be have length 4."
        // );

        match slice {
            // GPMF structural FourCC
            b"DEVC" => Ok(FourCC::DEVC),
            b"DVID" => Ok(FourCC::DVID),
            b"DVNM" => Ok(FourCC::DVNM),
            b"STRM" => Ok(FourCC::STRM),
            b"STNM" => Ok(FourCC::STNM),
            b"RMRK" => Ok(FourCC::RMRK),
            b"SCAL" => Ok(FourCC::SCAL),
            b"SIUN" => Ok(FourCC::SIUN),
            b"UNIT" => Ok(FourCC::UNIT),
            b"TYPE" => Ok(FourCC::TYPE),
            b"TSMP" => Ok(FourCC::TSMP),
            b"TIMO" => Ok(FourCC::TIMO),
            b"EMPT" => Ok(FourCC::EMPT),

            // Device/data specific FourCC
            b"AALP" => Ok(FourCC::AALP),
            b"ACCL" => Ok(FourCC::ACCL),
            b"ALLD" => Ok(FourCC::ALLD),
            b"CORI" => Ok(FourCC::CORI),
            b"DISP" => Ok(FourCC::DISP),
            b"FACE" => Ok(FourCC::FACE),
            b"FCNM" => Ok(FourCC::FCNM),
            b"GPS5" => Ok(FourCC::GPS5),
            b"GPSF" => Ok(FourCC::GPSF),
            b"GPSP" => Ok(FourCC::GPSP),
            b"GPSU" => Ok(FourCC::GPSU),
            b"GPSA" => Ok(FourCC::GPSA),
            b"GRAV" => Ok(FourCC::GRAV),
            b"GYRO" => Ok(FourCC::GYRO),
            b"HUES" => Ok(FourCC::HUES),
            b"IORI" => Ok(FourCC::IORI),
            b"ISOE" => Ok(FourCC::ISOE),
            b"ISOG" => Ok(FourCC::ISOG),
            b"LSKP" => Ok(FourCC::LSKP),
            b"MAGN" => Ok(FourCC::MAGN),
            b"MSKP" => Ok(FourCC::MSKP),
            b"MWET" => Ok(FourCC::MWET),
            b"ORIN" => Ok(FourCC::ORIN),
            b"ORIO" => Ok(FourCC::ORIO),
            b"MTRX" => Ok(FourCC::MTRX),
            b"SCEN" => Ok(FourCC::SCEN),
            b"SHUT" => Ok(FourCC::SHUT),
            b"SROT" => Ok(FourCC::SROT),
            b"STMP" => Ok(FourCC::STMP),
            b"UNIF" => Ok(FourCC::UNIF),
            b"WBAL" => Ok(FourCC::WBAL),
            b"WNDM" => Ok(FourCC::WNDM),
            b"WRGB" => Ok(FourCC::WRGB),
            b"YAVG" => Ok(FourCC::YAVG),

            // Content FourCC
            b"MSLV" => Ok(FourCC::MSLV),
            // Scene classifications, Hero7 Black only? Not in Hero8+9
            b"SNOW" => Ok(FourCC::SNOW),
            b"URBA" => Ok(FourCC::URBA),
            b"INDO" => Ok(FourCC::INDO),
            b"WATR" => Ok(FourCC::WATR),
            b"VEGE" => Ok(FourCC::VEGE),
            b"BEAC" => Ok(FourCC::BEAC),

            // MP4 user data atom (`udta`) only
            b"FIRM" => Ok(FourCC::FIRM),
            b"LENS" => Ok(FourCC::LENS),
            b"CAME" => Ok(FourCC::CAME),
            b"SETT" => Ok(FourCC::SETT),
            b"AMBA" => Ok(FourCC::AMBA),
            b"MUID" => Ok(FourCC::MUID),
            b"HMMT" => Ok(FourCC::HMMT),
            b"BCID" => Ok(FourCC::BCID),
            b"GUMI" => Ok(FourCC::GUMI),

            // JPEG GPMF FourCC
            b"MINF" => Ok(FourCC::MINF),

            // GoPro MP4 udta atom contains undocumented
            // GPMF data that is zero padded,
            // used as check for breaking parse loop
            b"\0" | b"\0\0\0\0" => Ok(Self::Invalid),

            // Undocumented FourCC
            _ => Ok(FourCC::Other(String::from_utf8_lossy(slice).to_string())),
        }
    }

    /// Generate FourCC enum from `&str`.
    pub fn from_str(fourcc: &str) -> Self {
        // NOTE Could be ISO8859-1 values that fit in single byte rather than standard ASCII
        assert_eq!(
            fourcc.chars().count(),
            4,
            "FourCC must be an ASCII string with length 4."
        );

        match fourcc.trim() {
            // GPMF structural FourCC
            "DEVC" => FourCC::DEVC,
            "DVID" => FourCC::DVID,
            "DVNM" => FourCC::DVNM,
            "STRM" => FourCC::STRM,
            "STNM" => FourCC::STNM,
            "RMRK" => FourCC::RMRK,
            "SCAL" => FourCC::SCAL,
            "SIUN" => FourCC::SIUN,
            "UNIT" => FourCC::UNIT,
            "TYPE" => FourCC::TYPE,
            "TSMP" => FourCC::TSMP,
            "TIMO" => FourCC::TIMO,
            "EMPT" => FourCC::EMPT,

            // Device/data specific FourCC
            "AALP" => FourCC::AALP,
            "ACCL" => FourCC::ACCL,
            "ALLD" => FourCC::ALLD,
            "CORI" => FourCC::CORI,
            "DISP" => FourCC::DISP,
            "FACE" => FourCC::FACE,
            "FCNM" => FourCC::FCNM,
            "GPS5" => FourCC::GPS5,
            "GPSF" => FourCC::GPSF,
            "GPSP" => FourCC::GPSP,
            "GPSU" => FourCC::GPSU,
            "GPSA" => FourCC::GPSA,
            "GRAV" => FourCC::GRAV,
            "GYRO" => FourCC::GYRO,
            "HUES" => FourCC::HUES,
            "IORI" => FourCC::IORI,
            "ISOE" => FourCC::ISOE,
            "ISOG" => FourCC::ISOG,
            "LSKP" => FourCC::LSKP,
            "MAGN" => FourCC::MAGN,
            "MSKP" => FourCC::MSKP,
            "MWET" => FourCC::MWET,
            "ORIN" => FourCC::ORIN,
            "ORIO" => FourCC::ORIO,
            "MTRX" => FourCC::MTRX,
            "SCEN" => FourCC::SCEN,
            "SHUT" => FourCC::SHUT,
            "SROT" => FourCC::SROT,
            "STMP" => FourCC::STMP,
            "UNIF" => FourCC::UNIF,
            "WBAL" => FourCC::WBAL,
            "WNDM" => FourCC::WNDM,
            "WRGB" => FourCC::WRGB,
            "YAVG" => FourCC::YAVG,

            // Content FourCC
            "MSLV" => FourCC::MSLV,
            // Scene classifications, Hero7 Black only? Not in Hero8+9
            "SNOW" => FourCC::SNOW,
            "URBA" => FourCC::URBA,
            "INDO" => FourCC::INDO,
            "WATR" => FourCC::WATR,
            "VEGE" => FourCC::VEGE,
            "BEAC" => FourCC::BEAC,

            // MP4 user data atom (`udta`) only
            "FIRM" => FourCC::FIRM,
            "LENS" => FourCC::LENS,
            "CAME" => FourCC::CAME,
            "SETT" => FourCC::SETT,
            "AMBA" => FourCC::AMBA,
            "MUID" => FourCC::MUID,
            "HMMT" => FourCC::HMMT,
            "BCID" => FourCC::BCID,
            "GUMI" => FourCC::GUMI,

            // JPEG GPMF FourCC
            "MINF" => FourCC::MINF,

            // Undocumented FourCC
            _ => FourCC::Other(fourcc.to_owned()),
        }
    }

    /// Generate `String` from `FourCC`.
    pub fn to_str(&self) -> &str {
        match self {
            // GPMF structural FourCC
            FourCC::DEVC => "DEVC",
            FourCC::DVID => "DVID",
            FourCC::DVNM => "DVNM",
            FourCC::STRM => "STRM",
            FourCC::STNM => "STNM",
            FourCC::RMRK => "RMRK",
            FourCC::SCAL => "SCAL",
            FourCC::SIUN => "SIUN",
            FourCC::UNIT => "UNIT",
            FourCC::TYPE => "TYPE",
            FourCC::TSMP => "TSMP",
            FourCC::TIMO => "TIMO",
            FourCC::EMPT => "EMPT",

            // Device/data specific FourCC
            FourCC::AALP => "AALP",
            FourCC::ACCL => "ACCL",
            FourCC::ALLD => "ALLD",
            FourCC::CORI => "CORI",
            FourCC::DISP => "DISP",
            FourCC::FACE => "FACE",
            FourCC::FCNM => "FCNM",
            FourCC::GPS5 => "GPS5",
            FourCC::GPSF => "GPSF",
            FourCC::GPSP => "GPSP",
            FourCC::GPSU => "GPSU",
            FourCC::GPSA => "GPSA",
            FourCC::GRAV => "GRAV",
            FourCC::GYRO => "GYRO",
            FourCC::HUES => "HUES",
            FourCC::IORI => "IORI",
            FourCC::ISOE => "ISOE",
            FourCC::ISOG => "ISOG",
            FourCC::LSKP => "LSKP",
            FourCC::MAGN => "MAGN",
            FourCC::MSKP => "MSKP",
            FourCC::MWET => "MWET",
            FourCC::ORIN => "ORIN",
            FourCC::ORIO => "ORIO",
            FourCC::MTRX => "MTRX",
            FourCC::SCEN => "SCEN",
            FourCC::SHUT => "SHUT",
            FourCC::SROT => "SROT",
            FourCC::STMP => "STMP",
            FourCC::UNIF => "UNIF",
            FourCC::WBAL => "WBAL",
            FourCC::WNDM => "WNDM",
            FourCC::WRGB => "WRGB",
            FourCC::YAVG => "YAVG",

            // Content FourCC
            // Mean Sea Level (altitude, in GPSA)
            FourCC::MSLV => "MSLV",
            // Scene classifications
            FourCC::SNOW => "SNOW",
            FourCC::URBA => "URBA",
            FourCC::INDO => "INDO",
            FourCC::WATR => "WATR",
            FourCC::VEGE => "VEGE",
            FourCC::BEAC => "BEAC",

            // MP4 user data atom (`udta`) only
            FourCC::FIRM => "FIRM",
            FourCC::LENS => "LENS",
            FourCC::CAME => "CAME",
            FourCC::SETT => "SETT",
            FourCC::AMBA => "AMBA",
            FourCC::MUID => "MUID",
            FourCC::HMMT => "HMMT",
            FourCC::BCID => "BCID",
            FourCC::GUMI => "GUMI",

            // JPEG GPMF FourCC
            FourCC::MINF => "MINF",

            // FourCC if [0, 0, 0, 0, ...] detected
            // (MP4 udta atom padding)
            FourCC::Invalid => "INVALID_FOURCC",

            // Undocumented FourCC
            FourCC::Other(s) => s,
        }
    }

    pub fn is_invalid(&self) -> bool {
        self == &FourCC::Invalid
    }
}
