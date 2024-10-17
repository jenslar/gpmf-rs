//! Core structure for GPMF raw data values.

use std::{default, io::{BufRead, Read, Seek}};

use binrw::{BinRead, BinReaderExt, BinResult};
use time::{format_description, PrimitiveDateTime};

use super::Header;
use crate::{gopro::Dvid, FourCC, GpmfError};

/// The core data type/wrapper for GPMF data types.
/// `Vec<T>` was chosen as inner value over `T` for (possibly misguided)
/// performance and boiler plate reasons.
///
/// A single GPMF stream may contain multiple `Value`s, i.e. `Vec<Value>`,
/// which is basically a `Vec<Vec<T>>`.
///
/// Type descriptions are edited versions of GoPro's own, see <https://github.com/gopro/gpmf-parser>.
///
/// Notes:
/// - Type `35`/`#` contains "Huffman compression STRM payloads. 4-CC <type><size><rpt> <data ...> is compressed as 4-CC '#'<new size/rpt> <type><size><rpt> <compressed data ...>" (see above GitHub repo).
/// It is currently parsed into `Vec<u8>`, but no further decoding or processing is implemented.
/// - Strings except `Value::Utf8()` variant map to ISO8859-1 as a single-byte (0-255) extension of ascii. I.e. `String::from_utf8(Vec<u8>)` would be incorrect or fail for values above 127. See <https://github.com/gopro/gpmf-parser/issues/143#issuecomment-952125684>. `u8 as char` is used as a workaround to produce a valid UTF-8 string.
///
/// For the original C source, see:
/// <https://github.com/gopro/gpmf-parser/blob/420930426c00a2ef3158847f967aed2acb2b06c1/GPMF_common.h>
#[derive(Debug, Clone, PartialEq, Default)]
pub enum Value {
    /// Ascii: c/99, single byte 'c' style ASCII character string, char.
    /// Optionally NULL terminated. Size/repeat sets the length.
    /// NOTE: Strings are defined as ISO8859-1, single-byte "extended ascii", 0-255,
    /// which means `String::from_utf8(Vec<u8>)` will fail for values above 127.
    /// The actual decimal value can be used as `212_u8 as char`.
    /// See <https://github.com/gopro/gpmf-parser/issues/143#issuecomment-952125684>
    String(String),
    /// F/70, 32-bit four character key -- FourCC, char fourcc[4]
    FourCC(String),
    /// G/71, 128-bit ID (like UUID), uint8_t guid[16]
    Uuid(Vec<u8>),
    /// u/117, UTF-8 formatted text string.
    /// As the character storage size varies, the size is in bytes, not UTF characters.
    Utf8(String),
    /// U/85, UTC Date and Time string, char utcdate[16].
    /// Date + UTC Time format yymmddhhmmss.sss - (years 20xx covered)
    /// e.g., "181007105554.644"
    Datetime(String),
    /// b/98, single byte signed integer, int8_t, -128 to 127
    Sint8(Vec<i8>),
    /// B/66, single byte unsigned integer, uint8_t	0 to 255
    Uint8(Vec<u8>),
    /// s/115, 16-bit signed integer, int16_t, -32768 to 32768
    Sint16(Vec<i16>),
    /// S/83, 16-bit unsigned integer, uint16_t, 0 to 65536
    Uint16(Vec<u16>),
    /// l/108, 32-bit signed integer, int32_t
    Sint32(Vec<i32>),
    /// L/76, 32-bit unsigned integer, uint32_t
    Uint32(Vec<u32>),
    /// f/102, 32-bit float (IEEE 754), float
    Float32(Vec<f32>),
    // https://stackoverflow.com/questions/8638792/how-to-convert-packed-integer-16-16-fixed-point-to-float
    // https://en.wikipedia.org/wiki/Q_(number_format)
    /// q/113, 32-bit Q Number Q15.16, uint32_t,
    /// 16-bit integer (A) with 16-bit fixed point (B) for A.B value.
    Qint32(Vec<u32>),
    /// j/106, 64-bit signed number, int64_t
    Sint64(Vec<i64>),
    /// J/74, 64-bit unsigned number, uint64_t
    Uint64(Vec<u64>),
    /// d/100, 64-bit double precision (IEEE 754), double
    Float64(Vec<f64>),
    /// Q/81, 64-bit Q Number Q31.32, uint64_t,
    /// 32-bit integer (A) with 32-bit fixed point (B) for A.B value.
    Qint64(Vec<u64>),
    /// ?/63, data structure is complex, meaning it contains other `Value`s.
    /// Defined by a preceding message with fourcc TYPE.
    /// Nesting is never deeper than one level,
    /// i.e. a complex type will never contain another complex type.
    Complex(Vec<Box<Self>>),
    /// #/35 Huffman compression STRM payloads.
    /// `4-CC <type><size><rpt> <data ...>` is compressed as
    /// `4-CC '#'<new size/rpt> <type><size><rpt> <compressed data ...>`.
    Compressed(Vec<u8>),
    /// 0/null, Nested metadata/container, e.g. DEVC, STRM.
    Nested,
    #[default]
    /// Empty message, e.g. when repeats or basesize is 0
    Empty,
    /// For erratic/corrupted data values that could not be read
    Invalid,
}

impl Value {
    /// Reads repeated data type `T` in Big Endian (all GPMF data is BE),
    /// into `Vec<T>`.
    ///
    /// With e.g. `Header::basetype = 76/L` (`u32`) we may have:
    /// - `Header::basesize = 12` (in bytes)
    /// - `std::mem::size_of::<u32>() = 4`
    ///
    /// This yields `12 / std::mem::size_of::<u32>() = 3`,
    /// i.e. one array with 3 `u32` values: `[1_u32, 2_u32, 3_u32]`
    // fn read<T>(
    fn read<T, R: Read + BufRead + Seek>(
        reader: &mut R,
        header: &Header,
    ) -> BinResult<Vec<T>>
        where
            // R: BinRead + Read + Seek,
            T: BinRead,
            <T as BinRead>::Args<'static>: Sized + Clone + Default
    {
        // Determine number of values, via type mem size.
        // May miss values or ignore insufficient data error
        // if integer division results in remainder?
        let size = std::mem::size_of::<T>();
        let range = 0..header.basesize as usize / size;

        // Below fails with
        // "BinReadError(Io(Error { kind: UnexpectedEof, message: "failed to fill whole buffer" })"
        // indicates corrupt data or implementation error?
        // IMPL ERROR header.repeats contain weird values...
        // using repeats yields very different result compared to using size_of above???
        // header implementation incorrect?
        // let range2 = 0..header.repeats as usize;
        // let range = 0 .. header.repeats as usize - 1;

        // println!("RANGE {range:?} | RANGE2 {range2:?}");

        range.into_iter().map(|_| reader.read_be::<T>()).collect()
    }

    /// Reads and maps ISO8859-1 single-byte values to a UTF-8 string.
    /// Ignores `null` characters.
    // fn from_iso8859_1(reader: &mut BufReader<File>, header: &Header) -> Result<String, GpmfError> {
    fn from_iso8859_1<R: Read + BufRead + Seek>(reader: &mut R, header: &Header) -> Result<String, GpmfError> {
        let bytes = Self::read::<u8, R>(reader, header)?;
        Ok(bytes
            .iter()
            .filter_map(|c| if c != &0 { Some(*c as char) } else { None })
            .collect())
    }

    /// Read and map byte values to a string if these correspond to valid UTF-8.
    // fn from_utf8(reader: &mut BufReader<File>, header: &Header) -> Result<String, GpmfError> {
    // fn from_utf8<R: Read + BufRead + Seek>(reader: &mut R, header: &Header) -> Result<String, GpmfError> {
    fn from_utf8<R: Read + BufRead + Seek>(reader: &mut R, header: &Header) -> Result<String, GpmfError> {
        let bytes = Self::read::<u8, R>(reader, header)?;
        String::from_utf8(bytes).map_err(|e| e.into())
    }

    /// Generates new `Value` enum from cursor. Supports complex types.
    ///
    /// > IMPORTANT: For GPMF/MP4 32-alignment is necessary. This has
    /// > to be done as the final step per GPMF stream (`Stream`)
    /// > outside of this method, since a single GPMF stream may translate
    /// > into multiple `Value` enums. Byte streams corresponding to multiple
    /// > `Value` enums in direct sucession are not 32-bit aligned.
    /// > The cursor position must always be 32-aligned before
    /// > reading GPMF data into another `Stream`, but NOT
    /// > between reading multiple `Value`s inside the same `Stream`.
    // pub(crate) fn new(
    //     reader: &mut BufReader<File>,
    //     header: &Header,
    //     complextype: Option<&str>,
    // pub(crate) fn new<R: Read + BufRead + Seek>(
    pub(crate) fn new<R: Read + BufRead + Seek>(
        reader: &mut R,
        header: &Header,
        complextype: Option<&str>,
    ) -> Result<Self, GpmfError> {
        let values = match header.basetype {
            b'b' => Self::Sint8(Self::read::<i8, R>(reader, header)?),
            b'B' => Self::Uint8(Self::read::<u8, R>(reader, header)?),
            b's' => Self::Sint16(Self::read::<i16, R>(reader, header)?),
            b'S' => Self::Uint16(Self::read::<u16, R>(reader, header)?),
            b'l' => Self::Sint32(Self::read::<i32, R>(reader, header)?),
            b'L' => Self::Uint32(Self::read::<u32, R>(reader, header)?),
            b'f' => Self::Float32(Self::read::<f32, R>(reader, header)?),
            b'd' => Self::Float64(Self::read::<f64, R>(reader, header)?),
            b'j' => Self::Sint64(Self::read::<i64, R>(reader, header)?),
            b'J' => Self::Uint64(Self::read::<u64, R>(reader, header)?),
            b'q' => Self::Qint32(Self::read::<u32, R>(reader, header)?),
            b'Q' => Self::Qint64(Self::read::<u64, R>(reader, header)?),
            // NOTE: 'String's are defined as ISO8859-1: i.e. single-byte "extended ascii", 0-255,
            // which means `String::from_utf8(Vec<u8>)` fails for values above 127.
            // The actual int/decimal values are the same however so using `u8 as char` works.
            b'c' => Self::String(Self::from_iso8859_1(reader, header)?),
            // FOURCC has total len 4, assert?
            b'F' => Self::FourCC(Self::from_iso8859_1(reader, header)?),
            // Explicit UTF8 string so must validate.
            b'u' => Self::Utf8(Self::from_utf8(reader, header)?),
            // DATETIME has total len 16 (ascii), assert?
            b'U' => Self::Datetime(Self::from_iso8859_1(reader, header)?),
            // UUID has total len 16, uint8_t, [u8; 16]
            b'G' => Self::Uuid(Self::read::<u8, R>(reader, header)?),
            // Huffman compression STRM payloads (just raw data, no decode)
            b'#' => Self::Compressed(Self::read::<u8, R>(reader, header)?),
            // Complex basetype, a dynamic, composite basetype that combines other basetypes.
            b'?' => {
                // Assumed that complex type contains a single ASCII basetype value/char
                // for each basetype specified. That is, basesize should always be equal
                // to bytesize of type, e.g. 4 for 32-bit value.
                // Should only go one recursive level deep.
                if let Some(types) = complextype {
                    // let complex: Result<Vec<Box<Value>>, GpmfError> = types.as_bytes().iter()
                    //     .map(|t| Self::new(cursor, &header.complex(t), None))
                    //     .map(|v| Box::new(v))
                    //     .collect();

                    let mut complex: Vec<Box<Self>> = Vec::new();

                    for t in types.as_bytes().iter() {
                        // Convert header with type `?` to header with specific type.
                        let hdr = header.convert(t);

                        let value = Self::new(reader, &hdr, None)?;

                        complex.push(Box::new(value));
                    }

                    Self::Complex(complex)
                } else {
                    return Err(GpmfError::MissingComplexType);
                }
            }
            // 0, NULL, containers (DEVC, STRM)
            0 => Self::Nested,
            b => return Err(GpmfError::UnknownBaseType(b)),
        };

        Ok(values)
    }

    pub fn debug(&self) -> &dyn std::fmt::Debug {
        match self {
            Self::String(v) => v,
            Self::Utf8(v) => v,
            Self::FourCC(v) => v,
            Self::Uuid(v) => v,
            Self::Datetime(v) => v,
            Self::Sint8(v) => v,
            Self::Uint8(v) => v,
            Self::Sint16(v) => v,
            Self::Uint16(v) => v,
            Self::Sint32(v) => v,
            Self::Uint32(v) => v,
            Self::Float32(v) => v,
            Self::Qint32(v) => v,
            Self::Sint64(v) => v,
            Self::Uint64(v) => v,
            Self::Float64(v) => v,
            Self::Qint64(v) => v,
            // Self::Complex(c) => c.iter().map(|b| b.value()),
            Self::Complex(c) => c,
            Self::Compressed(c) => c,
            v @ Self::Nested => v,
            v @ Self::Invalid => v,
            v @ Self::Empty => v,
        }
    }
}

// Conversions
impl AsRef<Self> for Value {
    fn as_ref(&self) -> &Self {
        self
    }
}

impl Into<Option<String>> for &Value {
    fn into(self) -> Option<String> {
        match self {
            Value::String(s) | Value::Datetime(s) | Value::FourCC(s) | Value::Utf8(s) => {
                Some(s.to_owned())
            }
            _ => None,
        }
    }
}

impl Into<Option<Dvid>> for &Value {
    fn into(self) -> Option<Dvid> {
        match self {
            Value::Uint32(d) => Some(Dvid::Uint32(d.first().map(|v| *v)?)),
            Value::FourCC(d) => Some(Dvid::FourCC(FourCC::from_str(d))),
            _ => None,
        }
    }
}

impl Into<Option<PrimitiveDateTime>> for &Value {
    fn into(self) -> Option<PrimitiveDateTime> {
        match self {
            Value::Datetime(dt) => {
                // 'time' crate does not parse two-digit years for ambiguity reasons.
                // See: https://github.com/time-rs/time/discussions/459
                // GPMF only covers years 2000+ so prefixing datetime string with "20"
                // before parse should be ok.
                // e.g. "181007105554.644" -> "20181007105554.644"
                let format = format_description::parse(
                    "[year][month][day][hour][minute][second].[subsecond]",
                )
                .ok()?;
                PrimitiveDateTime::parse(&format!("20{dt}"), &format).ok()
            }
            _ => None,
        }
    }
}

impl Into<Option<u16>> for &Value {
    fn into(self) -> Option<u16> {
        match self {
            Value::Uint16(n) => n.first().cloned(),
            _ => None,
        }
    }
}

impl Into<Option<u32>> for &Value {
    fn into(self) -> Option<u32> {
        match self {
            Value::Uint32(n) => n.first().cloned(),
            _ => None,
        }
    }
}

impl Into<Option<Vec<f64>>> for &Value {
    fn into(self) -> Option<Vec<f64>> {
        match self {
            Value::Sint8(n) => Some(n.iter().map(|v| f64::from(*v)).collect()),
            Value::Uint8(n) => Some(n.iter().map(|v| f64::from(*v)).collect()),
            Value::Sint16(n) => Some(n.iter().map(|v| f64::from(*v)).collect()),
            Value::Uint16(n) => Some(n.iter().map(|v| f64::from(*v)).collect()),
            Value::Sint32(n) => Some(n.iter().map(|v| f64::from(*v)).collect()),
            Value::Uint32(n) => Some(n.iter().map(|v| f64::from(*v)).collect()),
            Value::Float32(n) => Some(n.iter().map(|v| f64::from(*v)).collect()),
            Value::Uint64(n) => Some(n.iter().map(|v| *v as f64).collect::<Vec<f64>>()),
            Value::Sint64(n) => Some(n.iter().map(|v| *v as f64).collect::<Vec<f64>>()),
            Value::Float64(n) => Some(n.to_owned()),
            Value::Qint32(n) => {
                // Q15.16 format -> div by 2^15
                Some(
                    n.iter()
                        .map(|v| *v as f64 / (2_u16).pow(15) as f64)
                        .collect::<Vec<f64>>(),
                )
            }
            Value::Qint64(n) => {
                // Q31.32 format -> div by 2^31
                Some(
                    n.iter()
                        .map(|v| *v as f64 / (2_u32).pow(31) as f64)
                        .collect::<Vec<f64>>(),
                )
            }
            Value::Complex(n) => Some(
                n.iter()
                    .filter_map(|v| v.as_ref().into())
                    .collect::<Vec<f64>>(),
            ),
            _ => None,
        }
    }
}

impl From<&Value> for Option<f64> {
    fn from(value: &Value) -> Option<f64> {
        match value {
            Value::Sint8(n) => n.first().map(|v| f64::from(*v)),
            Value::Uint8(n) => n.first().map(|v| f64::from(*v)),
            Value::Sint16(n) => n.first().map(|v| f64::from(*v)),
            Value::Uint16(n) => n.first().map(|v| f64::from(*v)),
            Value::Sint32(n) => n.first().map(|v| f64::from(*v)),
            Value::Uint32(n) => n.first().map(|v| f64::from(*v)),
            Value::Float32(n) => n.first().map(|v| f64::from(*v)),
            Value::Uint64(n) => n.first().map(|v| *v as f64),
            Value::Sint64(n) => n.first().map(|v| *v as f64),
            Value::Float64(n) => n.first().cloned(),
            // Q15.16 format -> div by 2^15
            Value::Qint32(n) => n.first().map(|v| *v as f64 / (2_u16).pow(15) as f64),
            // Q31.32 format -> div by 2^31
            Value::Qint64(n) => n.first().map(|v| *v as f64 / (2_u32).pow(31) as f64),
            _ => None,
        }
    }
}
