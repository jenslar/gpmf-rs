//! Core data structure for GPMF streams, containing either more streams or raw values.

use std::io::{Cursor, Seek, SeekFrom};

use crate::{DataType, GpmfError, gopro::Dvid};
use super::{FourCC, Header, Value, Timestamp};

/// Core struct that preserves the GPMF structure.
/// Contains either more `Stream`s,
/// i.e. a container/nested stream (`Header.basetype == 0`),
/// or data values.
#[derive(Debug, Clone, PartialEq)]
pub struct Stream {
    /// Stream header
    pub header: Header,
    /// Child streams
    pub streams: StreamType,
    /// Relative timestamps.
    /// Duration since video start and "sample duration"
    /// of stream.
    pub time: Option<Timestamp>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum StreamType {
    /// Container (Header::basetype = 0). Contains more `Stream`s.
    Nested(Box<Vec<Stream>>),
    /// Terminal `Stream` nodes. Contain data.
    Values(Vec<Value>),
}

impl Stream {
    /// Parse GPMF chunk corresponding to one or more `Stream`s recursively.
    /// For GoPro MP4-files use `Stream::compile()` instead.
    /// 
    /// For GPMF data extracted with e.g. FFmpeg, the cursor will initially
    /// correspond to the full data load (i.e. all `DEVC` containers),
    /// whereas for MP4 files it will initially correspond to the number
    /// of `DEVC` containers at a specific byte offeset in the MP4.
    /// 
    /// `read_limit` is used to avoid embeddning nested streams/containters that
    /// follow directly after each other into other nested streams.
    /// Since `Stream::new()` calls itself recusively each container will otherwise
    /// emded the next until end of cursor.
    // pub fn new(cursor: &mut Cursor<Vec<u8>>, read_limit: Option<usize>) -> Result<Vec<Self>, GpmfError> {
    pub fn new(
        cursor: &mut Cursor<Vec<u8>>,
        read_limit: Option<usize>,
        debug: bool
    ) -> Result<Vec<Self>, GpmfError> {

        // Type definitions (BaseType::TYPE) for BaseType::COMPLEX. 'TYPE' must precede 'COMPLEX',
        // e.g. for TYPE: "cF" -> BaseType::COMPLEX(Vec<BaseType::ASCII(_), BaseType::FOURCC(_))
        let mut complex: Option<String> = None;

        let mut streams: Vec<Self> = Vec::new();

        // End position in bytes, with optional read limit equal to current
        // cursor position + set limit value. To avoid recusively
        // embedding containers that follow directly after each other.
        let len = match read_limit {
            Some(l) => cursor.position() as usize + l,
            None => cursor.get_ref().len()
        };

        // TODO check if read limit should substract header size above or below? works any way right now...

        while cursor.position() < len as u64 {
            // 8 byte header
            let header = Header::new(cursor)?;

            if debug {
                println!("@{} {header:3?} | LEN: {}", cursor.position(), header.size(true) + 8); // position is only offset from start of current DEVC, not entire MP4
            }
            
            // `FourCC::Invalid` currently a check for `&[0,0,0,0, ...]`
            // to be able to break loops in GPMF streams that end with 0-padding.
            // So far specific to GPMF streams in MP4 `udta` atoms.
            // This will only occur for the final stream/DEVC in udta,
            // so the `Header.pad` value is not needed.
            if header.is_invalid() {
                break
            }
            
            let pad = header.pad;

            match header.basetype {
                // 0 = Container/nested stream
                0 => {
                    // Create new stream after header offset if GPMF chunk is a container.
                    // Set byte read limit to avoid embedding containers/0 in each other indefinitely if they
                    // follow directly after each other.
                    // let stream = Self::new(cursor, Some(header.size(false) as usize))?;
                    let stream = Self::new(cursor, Some(header.size(false) as usize), debug)?;

                    streams.push(Self{
                        header,
                        streams: StreamType::Nested(Box::new(stream)),
                        time: None
                    })
                },

                // Anything else will contain data values
                _ => {
                    let mut values: Vec<Value> = Vec::new();

                    // Parse Values in stream.
                    // First check if stream has no data,
                    // to avoid parsing further.
                    if header.basesize * header.repeats == 0 {
                        values.push(Value::Empty)
                    } else {
                        for _ in 0..header.repeats {
                            values.push(Value::new(cursor, &header, complex.as_deref())?);
                        }
                    }

                    // Check for complex type definitions
                    if header.fourcc == FourCC::TYPE {
                        // TYPE contains a single string for complex types with GPMF type definitions,
                        // e.g. "bLL" denotes that there are three single values `i8`, `u32`, `u32`.
                        complex = values.first().and_then(|v| v.into());
                    }

                    streams.push(Self{
                        header,
                        streams: StreamType::Values(values),
                        time: None
                    })

                }
            }

            // Seek forward equal to header padding value (0-3 bytes),
            // past 0 padding, for 32-bit alignment.
            cursor.seek(SeekFrom::Current(pad as i64))?;
        }

        Ok(streams)
    }

    /// Set relative timestamp for GPMF stream.
    pub fn set_time(&mut self, time: &Timestamp) {
        self.time = Some(time.to_owned());
    }

    /// Returns original size in bytes, before being parsed, for current `Stream`.
    /// `aligned = true` returns the size including 32-bit alignment padding.
    /// Not all streams are padded.
    pub fn size(&self, aligned: bool) -> u32 {
        self.header.size(aligned)
    }

    /// Return number of linked `Stream`s (if a nested stream),
    /// or number of `Value`s (if a terminal stream, i.e. contains data).
    pub fn len(&self) -> usize {
        // or Option<usize>?
        match &self.streams {
            StreamType::Nested(s) => s.len(),
            StreamType::Values(v) => v.len(),
        }
    }

    /// Depending on `StreamType` type. For `StreamType::Nested` check if `Stream` links further
    /// `Stream`s, i.e. if `Stream` has any child nodes.
    /// For `StreamType::Values` check if any data is contained.
    pub fn is_empty(&self) -> bool {
        match &self.streams {
            StreamType::Nested(s) => s.is_empty(),
            StreamType::Values(v) => v.is_empty(),
        }
    }

    /// Returns Four CC for stream.
    pub fn fourcc(&self) -> &FourCC {
        &self.header.fourcc
    }

    /// Returns `true` if specified Four CC
    /// matches that of current `Stream`.
    pub fn has_fourcc(&self, fourcc: &FourCC) -> bool {
        self.fourcc() == fourcc
    }

    /// Returns values for current `Stream`, if present.
    /// `None` is returned if `StreamType::Nested`.
    pub fn values(&self) -> Option<Vec<Value>> {
        match &self.streams {
            StreamType::Nested(_) => None,
            StreamType::Values(v) => Some(v.to_owned()),
        }
    }

    /// Returns first `Value` in a terminal stream (i.e. contains values),
    /// `None` is returned if the stream is not a terminal node (i.e. contains more streams).
    pub fn first_value(&self) -> Option<&Value> {
        match &self.streams {
            StreamType::Values(v) => v.first(),
            _ => None,
        }
    }

    /// Returns last `Value` in a terminal stream (i.e. contains values),
    /// `None` is returned if the stream is not a terminal node (i.e. contains more streams).
    pub fn last_value(&self) -> Option<&Value> {
        match &self.streams {
            StreamType::Values(v) => v.last(),
            _ => None,
        }
    }

    /// Convenience method that iterates over numerical values in a terminal stream
    /// (i.e. contains values) and casts these as `Vec<Vec<f64>>`,
    /// where each "inner" `Vec<f64>` represents the values wrapped by a single `Value`.
    /// 
    /// For cases where `Value` is known to wrap multiple values.
    /// 
    /// `None` is returned if the stream is not a terminal node, or if the
    /// `Value` does not wrap numerical data.
    pub fn to_vec_f64(&self) -> Option<Vec<Vec<f64>>> {
        match &self.streams {
            StreamType::Nested(_) => None,
            StreamType::Values(v) => v.iter()
                .map(|b| b.into())
                // TODO need to implement Into for Value::Complex
                // .map(|b| if let Value::Complex(vec) = b {
                //     vec.iter().map(|v| (*v).as_ref().to_f64()).to_owned().to_vec_f64().flatten()
                // } else {
                //     b.into()
                // })
                .collect(),
        }
    }

    /// Convenience method that iterates over numerical values in a terminal stream
    /// (i.e. contains values) and casts these as `Vec<f64>`,
    /// 
    /// For cases where `Value` is known to only wrap a single value.
    /// 
    /// `None` is returned if the stream is not a terminal node, or if the
    /// `Value` does not wrap numerical data.
    pub fn to_f64(&self) -> Option<Vec<f64>> {
        match &self.streams {
            StreamType::Nested(_) => None,
            StreamType::Values(v) => v.iter()
                .map(|b| b.into())
                .collect(),
        }
    }

    /// Returns first `Stream` in a nested stream (i.e. contains more streams),
    /// `None` is returned if the stream is a terminal node (i.e. contains values).
    pub fn first_stream(&self) -> Option<&Stream> {
        match &self.streams {
            StreamType::Nested(s) => s.first(),
            _ => None,
        }
    }

    /// Returns last `Stream` in a nested stream (i.e. contains more streams),
    /// `None` is returned if the stream is a terminal node (i.e. contains values).
    pub fn last_stream(&self) -> Option<&Stream> {
        match &self.streams {
            StreamType::Nested(s) => s.last(),
            _ => None,
        }
    }

    // /// Returns the duration in milliseconds for the current stream if set.
    // /// For a more accurate media source/s duration, use `Gpmf::duration()`.
    // pub fn duration(&self) -> Option<u32> {
    //     Some(self.time.to_owned()?.relative)
    // }

    pub fn is_nested(&self) -> bool {
        matches!(self.streams, StreamType::Nested(_))
    }

    /// Find first stream with specified FourCC.
    /// Matches self and direct decendants.
    pub fn find(&self, fourcc: &FourCC) -> Option<&Self> {
        match self.has_fourcc(fourcc) {
            true => return Some(&self),
            false => {
                match &self.streams {
                    StreamType::Nested(streams) => {
                        streams.iter()
                            .find(|st| st.has_fourcc(fourcc))
                    }
                    StreamType::Values(_) => None
                }
            }
        }
    }

    // /// Find all stream with specified FourCC. Matches current stream
    // /// and direct decendants.
    // /// If current stream matches, a single `Stream` will be returned.
    // pub fn find_all(&self, fourcc: &FourCC) -> Vec<Self> {
    //     match self.has_fourcc(fourcc) {
    //         true => return vec![self.to_owned()],
    //         false => {
    //             if let StreamType::Nested(streams) = &self.streams {
    //                 streams.iter()
    //                     .filter(|st| st.has_fourcc(fourcc))
    //                     .cloned()
    //                     .collect::<Vec<_>>()
    //             } else {
    //                 Vec::new()
    //             }
    //         }
    //     }
    // }
    // pub fn find_all(&self, fourcc: &FourCC, recursive: bool) -> Vec<Self> {
        
    /// Find all stream with specified FourCC. Matches current stream
    /// and direct decendants.
    pub fn find_all(&self, fourcc: &FourCC) -> Vec<Self> {
        let mut streams: Vec<Stream> = Vec::new();
        match &self.streams {
            StreamType::Values(_) => return Vec::new(),
            StreamType::Nested(strms) => {
                // streams.append(
                //     &mut strms.iter()
                streams.extend(
                    strms.iter()
                        // .inspect(|s| println!("{:?}", s.fourcc()))
                        .filter(|s| s.has_fourcc(fourcc))
                        .cloned()
                        .collect::<Vec<_>>()
                )
                // // Check self for match
                // if self.has_fourcc(fourcc) {
                //     // return Some(self)
                //     streams.push(self.to_owned());
                //     // println!("SELF CHECK, PUSHED {fourcc:?}");
                // }

                // // Check child nodes for match, optionally recursive
                // for stream in strms.iter() {
                //     if stream.has_fourcc(fourcc) {
                //         // return Some(stream)
                //         streams.push(stream.to_owned());
                //         // println!("ITER CHECK, PUSHED {fourcc:?}");
                //     }

                //     // Check child nodes an additional level down inside loop for a recursive search
                //     // if recursive {
                //     //     match &stream.find(fourcc, recursive) {
                //     //         Some(s) => return Some(s.to_owned()),
                //     //         None => ()
                //     //     }
                //     // }
                // }

                // Return None if all child nodes are exhausted with no match
                // None
            }
        }
        streams
    }

    pub fn find_all2(&self, fourcc: &FourCC) -> Vec<Self> {
        match &self.streams {
            StreamType::Values(_) => vec![],
            StreamType::Nested(streams) => {
                streams.iter()
                    // .inspect(|s| println!("{:?}", s.fourcc()))
                    .filter(|s| s.has_fourcc(fourcc))
                    .cloned()
                    .collect()
            }
        }
    }

    /// Returns the human redable name of the stream
    /// if it is a `STRM`, (stored as string in `STNM`),
    /// otherwise `None` is returned.
    pub fn name(&self) -> Option<String> {
        self.find(&FourCC::STNM)
            .and_then(|strm| strm.first_value())
            .and_then(|val| val.into())
    }

    /// Returns duration relative to GPMF start if set.
    /// Should be close to video position.
    /// 
    /// > **Note:** All `Stream`s have timestamps derived from
    /// the original MP4 (at the `DEVC` container level).
    /// The current, official GPMF specification
    /// does not implement logging time stamps for individual data points.
    /// Thus, GPMF data extracted via e.g. `ffmpeg` or in the MP4 `udta` atom
    /// will not and can not have timestamps.
    pub fn time_relative(&self) -> Option<time::Duration> {
        // Some(self.time.as_ref()?.to_relative())
        self.time.as_ref().map(|t| t.relative)
    }

    /// Returns duration for current GPMF chunk if set.
    /// 
    /// > **Note:** All `Stream`s have timestamps derived from
    /// the original MP4 (at the `DEVC` container level).
    /// The current, official GPMF specification
    /// does not implement logging time stamps for individual data points,
    /// other than for `GPS9` devices (Hero11 and newer).
    /// Thus, GPMF data extracted via e.g. `ffmpeg` or in the MP4 `udta` atom
    /// will not and can not have timestamps.
    pub fn time_duration(&self) -> Option<time::Duration> {
        // Some(self.time.as_ref()?.to_duration())
        self.time.as_ref().map(|t| t.duration)
    }

    // pub fn time_duration_ms(&self) -> Option<time::Duration> {
    pub fn time_duration_ms(&self) -> Option<i128> {
        // Some(self.time.as_ref()?.to_duration())
        Some(self.time.as_ref()?.duration_ms())
    }

    /// Find first stream with specified `DataType`.
    pub fn filter(&self, content_type: &DataType) -> Vec<Self> {
        match &self.streams {
            StreamType::Values(_) => Vec::new(), // better way?
            StreamType::Nested(streams) => {
                streams.iter()
                    .filter_map(|s| {
                        if s.name() == Some(content_type.to_str().to_owned()) {
                            Some(Stream {
                                // Inherit timestamp from parent (only present in DEVC containers),
                                // since this is otherwise lost.
                                time: self.time.to_owned(),
                                ..s.to_owned()
                            })
                        } else {
                            None
                        }
                    })
                    .collect()
            }
        }
    }

    /// Find all streams with specified `DataType`.
    pub fn filter_all(
        &self,
        content_type: &DataType,
        recursive: bool
    ) -> Option<&Self> {
        match &self.streams {
            StreamType::Values(_) => return None,
            StreamType::Nested(streams) => {
                // Check self for match
                if self.name().as_deref() == Some(content_type.to_str()) {
                    return Some(self);
                }

                // Check child nodes for match, optionally recursive
                for stream in streams.iter() {
                    if self.name().as_deref() == Some(content_type.to_str()) {
                        return Some(stream);
                    }

                    // Check child nodes an additional level down inside loop for a recursive search
                    if recursive {
                        match &stream.filter_all(content_type, recursive) {
                            Some(s) => return Some(s.to_owned()),
                            None => (),
                        }
                    }
                }

                // Return None if all child nodes are exhausted with no match
                None
            }
        }
    }

    /// Returns Device ID (`DVID` stream) if input Stream is a `DEVC` container, or `DVID` stream.
    /// If you want to search for Device ID starting from an arbitrary `Stream`, try
    /// `Stream::find(&FourCC::DVID)` instead.
    pub fn device_id(&self) -> Option<Dvid> {
        match &self.fourcc() {
            FourCC::DVID => {
                self.first_value()
                    .and_then(|b| b.into())
                    // .and_then(|b| b.to_owned().into())
            }
            FourCC::DEVC => {
                self.find(&FourCC::DVID)
                    .and_then(|f| f.first_value())
                    .and_then(|b| b.into())
                    // .and_then(|b| b.to_owned().into())
            }
            _ => None,
        }
    }

    /// Returns Device Name (`DVNM` stream) if input Stream is a `DEVC` container, or `DVNM` stream.
    /// If you want to search for Device Name starting from an arbitrary `Stream`, try
    /// `Stream::find(&FourCC::DVNM)` instead.
    pub fn device_name(&self) -> Option<String> {
        match &self.fourcc() {
            FourCC::DVNM => {
                self.first_value()
                    .and_then(|b| b.into())
            }
            FourCC::DEVC => {
                self.find(&FourCC::DVNM)
                    .and_then(|f| f.first_value())
                    .and_then(|b| b.into())
            }
            _ => None,
        }
    }

    /// Print `Stream` contents.
    pub fn print(&self, count: Option<usize>, size: Option<usize>) {
        let cnt = count.unwrap_or(1);
        let sz = size.unwrap_or(self.len());

        let prefix = match &self.fourcc() {
            FourCC::DEVC => {
                format!("[{}/{}] ", cnt, sz)
            }
            FourCC::DVID | FourCC::DVNM | FourCC::STRM => "    ".to_owned(),
            _ => "      ".to_owned(),
        };

        println!(
            "{}{}, NAME: {:?}, TIME: {:?}",
            prefix,
            &self.header,
            &self.name(),
            &self.time
        );

        match &self.streams {
            StreamType::Nested(stream) => {
                stream.iter().for_each(|s| s.print(Some(cnt), Some(sz)))
            }
            StreamType::Values(values) => {
                for v in values.iter() {
                    println!("        {:?}", v.debug())
                }
            }
        }
    }
}