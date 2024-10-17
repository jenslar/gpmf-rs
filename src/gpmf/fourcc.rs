//! GPMF Four CC, i.e. general stream identifier.
//! Not all are covered or documented, hence `FourCC::Other(String)`.
//! `FourCC::Invalid` is there to check for zero padding in MP4 `udta` atom GPMF streams,
//! which will otherwise erronously be parsed as valid GPMF FourCC.

use std::io::{Read, BufRead, Seek};

use binrw::BinRead;

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
    /// HERO5Black - Hero10Black (logged, but deprecated for Hero11)
    /// 18Hz
    /// latitude, longitude, altitude (WGS 84), 2D ground speed, and 3D speed
    /// deg, deg, m, m/s, m/s
    GPS5,
    /// Hero11 Black and later
    /// lat, long, alt (WGS 84), 2D speed, 3D speed, days, seconds, DOP, fix
    /// deg, deg, m, m/s, m/s, days, seconds, NONE, NONE
    GPS9,
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
    /// MP4 `udta` global unique media ID
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

// impl BinRead for FourCC {
//     type Args<'a>;

//     fn read_options<R: Read + Seek>(
//         reader: &mut R,
//         endian: binrw::Endian,
//         args: Self::Args<'_>,
//     ) -> binrw::BinResult<Self> {
//         let mut buf: [u8; 4] = [0; 4];
//         reader.read::<[u8; 4]>(buf)?;
//         Ok(Self::from_arr(buf))
//     }
// }

impl FourCC {
    pub fn new<R: Read + BufRead + Seek>(reader: &mut R) -> Result<Self, GpmfError> {
        let mut buf = vec![0_u8; 4];
        let _len = reader.read_exact(&mut buf)?;
        Ok(Self::from_slice(&buf))
    }

    /// Generate FourCC enum from `&str`.
    // fn from_slice(slice: &[u8]) -> Result<Self, GpmfError> {
    fn from_slice(slice: &[u8]) -> Self {

        match slice {
            // GPMF structural FourCC
            b"DEVC" => FourCC::DEVC,
            b"DVID" => FourCC::DVID,
            b"DVNM" => FourCC::DVNM,
            b"STRM" => FourCC::STRM,
            b"STNM" => FourCC::STNM,
            b"RMRK" => FourCC::RMRK,
            b"SCAL" => FourCC::SCAL,
            b"SIUN" => FourCC::SIUN,
            b"UNIT" => FourCC::UNIT,
            b"TYPE" => FourCC::TYPE,
            b"TSMP" => FourCC::TSMP,
            b"TIMO" => FourCC::TIMO,
            b"EMPT" => FourCC::EMPT,

            // Device/data specific FourCC
            b"AALP" => FourCC::AALP,
            b"ACCL" => FourCC::ACCL,
            b"ALLD" => FourCC::ALLD,
            b"CORI" => FourCC::CORI,
            b"DISP" => FourCC::DISP,
            b"FACE" => FourCC::FACE,
            b"FCNM" => FourCC::FCNM,
            b"GPS5" => FourCC::GPS5,
            b"GPS9" => FourCC::GPS9,
            b"GPSF" => FourCC::GPSF,
            b"GPSP" => FourCC::GPSP,
            b"GPSU" => FourCC::GPSU,
            b"GPSA" => FourCC::GPSA,
            b"GRAV" => FourCC::GRAV,
            b"GYRO" => FourCC::GYRO,
            b"HUES" => FourCC::HUES,
            b"IORI" => FourCC::IORI,
            b"ISOE" => FourCC::ISOE,
            b"ISOG" => FourCC::ISOG,
            b"LSKP" => FourCC::LSKP,
            b"MAGN" => FourCC::MAGN,
            b"MSKP" => FourCC::MSKP,
            b"MWET" => FourCC::MWET,
            b"ORIN" => FourCC::ORIN,
            b"ORIO" => FourCC::ORIO,
            b"MTRX" => FourCC::MTRX,
            b"SCEN" => FourCC::SCEN,
            b"SHUT" => FourCC::SHUT,
            b"SROT" => FourCC::SROT,
            b"STMP" => FourCC::STMP,
            b"UNIF" => FourCC::UNIF,
            b"WBAL" => FourCC::WBAL,
            b"WNDM" => FourCC::WNDM,
            b"WRGB" => FourCC::WRGB,
            b"YAVG" => FourCC::YAVG,

            // Content FourCC
            b"MSLV" => FourCC::MSLV,
            // Scene classifications, Hero7 Black only? Not in Hero8+9
            b"SNOW" => FourCC::SNOW,
            b"URBA" => FourCC::URBA,
            b"INDO" => FourCC::INDO,
            b"WATR" => FourCC::WATR,
            b"VEGE" => FourCC::VEGE,
            b"BEAC" => FourCC::BEAC,

            // MP4 user data atom (`udta`) only
            b"FIRM" => FourCC::FIRM,
            b"LENS" => FourCC::LENS,
            b"CAME" => FourCC::CAME,
            b"SETT" => FourCC::SETT,
            b"AMBA" => FourCC::AMBA,
            b"MUID" => FourCC::MUID,
            b"HMMT" => FourCC::HMMT,
            b"BCID" => FourCC::BCID,
            b"GUMI" => FourCC::GUMI,

            // JPEG GPMF FourCC
            b"MINF" => FourCC::MINF,

            // GoPro MP4 udta atom contains undocumented
            // GPMF data that is zero padded,
            // used as check for breaking parse loop
            b"\0" | b"\0\0\0\0" => Self::Invalid,

            // Lossy UTF-8 does not work correctly for single-byte char above 127
            // (not standard ASCII)
            // _ => Self::Other(String::from_utf8_lossy(fourcc).to_string()),
            // Works for ranges 0-255
            _ => Self::Other(slice.iter().map(|n| *n as char).collect::<String>()),
        }
    }
    fn from_arr(arr: [u8; 4]) -> Self {
        // assert!(slice.len() == 4, "FourCC must be have length 4.");

        Self::from_slice(&arr)

        // match &slice {
        //     // GPMF structural FourCC
        //     b"DEVC" => FourCC::DEVC,
        //     b"DVID" => FourCC::DVID,
        //     b"DVNM" => FourCC::DVNM,
        //     b"STRM" => FourCC::STRM,
        //     b"STNM" => FourCC::STNM,
        //     b"RMRK" => FourCC::RMRK,
        //     b"SCAL" => FourCC::SCAL,
        //     b"SIUN" => FourCC::SIUN,
        //     b"UNIT" => FourCC::UNIT,
        //     b"TYPE" => FourCC::TYPE,
        //     b"TSMP" => FourCC::TSMP,
        //     b"TIMO" => FourCC::TIMO,
        //     b"EMPT" => FourCC::EMPT,

        //     // Device/data specific FourCC
        //     b"AALP" => FourCC::AALP,
        //     b"ACCL" => FourCC::ACCL,
        //     b"ALLD" => FourCC::ALLD,
        //     b"CORI" => FourCC::CORI,
        //     b"DISP" => FourCC::DISP,
        //     b"FACE" => FourCC::FACE,
        //     b"FCNM" => FourCC::FCNM,
        //     b"GPS5" => FourCC::GPS5,
        //     b"GPS9" => FourCC::GPS9,
        //     b"GPSF" => FourCC::GPSF,
        //     b"GPSP" => FourCC::GPSP,
        //     b"GPSU" => FourCC::GPSU,
        //     b"GPSA" => FourCC::GPSA,
        //     b"GRAV" => FourCC::GRAV,
        //     b"GYRO" => FourCC::GYRO,
        //     b"HUES" => FourCC::HUES,
        //     b"IORI" => FourCC::IORI,
        //     b"ISOE" => FourCC::ISOE,
        //     b"ISOG" => FourCC::ISOG,
        //     b"LSKP" => FourCC::LSKP,
        //     b"MAGN" => FourCC::MAGN,
        //     b"MSKP" => FourCC::MSKP,
        //     b"MWET" => FourCC::MWET,
        //     b"ORIN" => FourCC::ORIN,
        //     b"ORIO" => FourCC::ORIO,
        //     b"MTRX" => FourCC::MTRX,
        //     b"SCEN" => FourCC::SCEN,
        //     b"SHUT" => FourCC::SHUT,
        //     b"SROT" => FourCC::SROT,
        //     b"STMP" => FourCC::STMP,
        //     b"UNIF" => FourCC::UNIF,
        //     b"WBAL" => FourCC::WBAL,
        //     b"WNDM" => FourCC::WNDM,
        //     b"WRGB" => FourCC::WRGB,
        //     b"YAVG" => FourCC::YAVG,

        //     // Content FourCC
        //     b"MSLV" => FourCC::MSLV,
        //     // Scene classifications, Hero7 Black only? Not in Hero8+9
        //     b"SNOW" => FourCC::SNOW,
        //     b"URBA" => FourCC::URBA,
        //     b"INDO" => FourCC::INDO,
        //     b"WATR" => FourCC::WATR,
        //     b"VEGE" => FourCC::VEGE,
        //     b"BEAC" => FourCC::BEAC,

        //     // MP4 user data atom (`udta`) only
        //     b"FIRM" => FourCC::FIRM,
        //     b"LENS" => FourCC::LENS,
        //     b"CAME" => FourCC::CAME,
        //     b"SETT" => FourCC::SETT,
        //     b"AMBA" => FourCC::AMBA,
        //     b"MUID" => FourCC::MUID,
        //     b"HMMT" => FourCC::HMMT,
        //     b"BCID" => FourCC::BCID,
        //     b"GUMI" => FourCC::GUMI,

        //     // JPEG GPMF FourCC
        //     b"MINF" => FourCC::MINF,

        //     // GoPro MP4 udta atom contains undocumented
        //     // GPMF data that is zero padded,
        //     // used as check for breaking parse loop
        //     // b"\0" | b"\0\0\0\0" => Self::Invalid,
        //     b"\0\0\0\0" => Self::Invalid,

        //     // Lossy UTF-8 does not work correctly for single-byte char above 127
        //     // (not standard ASCII)
        //     // _ => Self::Other(String::from_utf8_lossy(fourcc).to_string()),
        //     // Works for ranges 0-255
        //     _ => Self::Other(slice.iter().map(|n| *n as char).collect::<String>()),
        // }
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
            FourCC::GPS9 => "GPS9",
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
