//! GPMF header that precedes each GPMF stream.

use std::{io::{BufRead, Seek, Read}, fmt};

use binrw::BinReaderExt;

use super::FourCC;
use crate::GpmfError;

/// GPMF header.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Header {
    /// FourCC.
    pub fourcc: FourCC,
    /// Base type.
    pub basetype: u8,
    /// Base size.
    /// Note: this is a `u8` in GPMF spec.
    /// Changed to `u16` for more convenient
    /// string parsing to avoid casting `u16` as `u8`.
    pub basesize: u16,
    pub repeats: u16,
    pub pad: u8,
}

impl fmt::Display for Header {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} basetype: {:4}/{}, basesize: {:2}, repeats: {:2}, pad: {}",
            self.fourcc.to_str(),
            self.basetype,
            self.basetype as char,
            self.basesize,
            self.repeats,
            self.pad,
        )
    }
}

impl Header {
    /// GPMF header.
    /// Layout:
    /// - 0-3: 32-bit FourCC (ascii string)
    /// - Structure:
    ///     - 4: Type (u8)
    ///     - 5: Structure size (u8 in spec, but stored as u16)
    ///     - 6-7: Repeat (u16)
    ///
    /// GPMF strings (e.g. STNM):
    /// Between older and newer GoPro devices, strings are
    /// sometimes structure size = 1, repeat X, somtimes size = X, repeat 1.
    /// To simplify unpacking/parsing strings ('c'/99), structure size is set to u16,
    /// to allow Type and structure size to switch places so that all string data
    /// becomes structure size X (byte length of utf-8 string), repeat 1, e.g. for older devices:
    /// `['G'] ['y'] ['r'] ['o']`, will instead be parsed into `['G', 'y', 'r', 'o']` -> String "Gyro".
    /// This is also true for Huffman encoded data loads, but these are currently not supported.
    // pub fn new(reader: &mut BufReader<File>) -> Result<Self, GpmfError> {
    // pub fn new<R: Read + BufRead + Seek>(reader: &mut R) -> Result<Self, GpmfError> {
    pub fn new<R: Read + BufRead + Seek>(reader: &mut R) -> Result<Self, GpmfError> {
        let fourcc = FourCC::new(reader)?;

        // check for "\0" and if found set header fourcc as invalid
        if fourcc.is_invalid() {
            return Ok(Self::default())
        }

        let basetype: u8 = reader.read_ne()?;

        // Temp valus for GPMF structure size.
        // Switch places between size and repeat if
        // size is 1.
        let tmp_basesize: u8 = reader.read_ne()?;
        let tmp_repeats: u16 = reader.read_be()?;

        // Check if structure size = 1 and basesize is a char,
        // and switch places if so to simplify string parsing.
        let (basesize, repeats) = match (basetype, tmp_basesize) {
            (b'c', 1) => (tmp_repeats, tmp_basesize as u16),
            _ => (tmp_basesize as u16, tmp_repeats)
        };

        // Set padding value for 32-bit alignment (0-3)
        let mut pad = 0;
        loop {
            match (basesize * repeats + pad as u16) % 4 {
                0 => break,
                _ => pad += 1,
            }
        }

        Ok(Self {
            fourcc,
            basetype,
            basesize,
            repeats,
            pad,
        })
    }

    /// Converts header to single-value header with specified type.
    /// Used for for complex type (`63`/`?`).
    pub fn convert(&self, basetype: &u8) -> Self {
        Self {
            basetype: *basetype,
            basesize: Self::baselen(basetype) as u16,
            ..self.to_owned()
        }
    }

    /// Returns `true` if header Four CC
    /// equals `[0, 0, 0, 0]`.
    pub fn is_invalid(&self) -> bool {
        self.fourcc == FourCC::Invalid
    }

    /// Get base length (same as `std::mem::size_of::<T>()`).
    /// Only used for COMPLEX BaseTypes.
    const fn baselen(basetype: &u8) -> u8 {
        match basetype {
            // Value::Sint8, Uint8, Ascii
            b'b' | b'B' | b'c' | 0 | b'?' => 1,

            // Value::Sint16, Uint16
            b's' | b'S' => 2,

            // Value::Sint32, Sint32, Uint32, Float32, Qint32, FourCC
            b'l' | b'L' | b'f' | b'q' | b'F' => 4,

            // Value::Float64, Sint64, Uint64, Qint64
            b'd' | b'j' | b'J' | b'Q' => 8,

            // Value::Datetime, Uuid
            b'U' | b'G' => 16,

            // nested data/0 (no direct values to parse), or unknown types
            _ => 1,
        }
    }

    /// Returns data size in bytes for the data the header precedes/describes.
    /// `aligned = true` returns 32-bit aligned/padded size.
    pub fn size(&self, aligned: bool) -> u32 {
        let size = self.basesize as u32 * self.repeats as u32;
        match aligned {
            true => size + self.pad as u32,
            false => size,
        }
    }
}
