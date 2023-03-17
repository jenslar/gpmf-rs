//! GoPro MP4 metadata logged in the user data atom `udta`.
//! 
//! GoPro embeds undocumented GPMF streams in the `udta` atom
//! that is also extracted.

use std::path::{Path, PathBuf};

use binread::{BinResult, BinReaderExt};
use mp4iter::{FourCC, Mp4, UdtaField};

use crate::{Stream, GpmfError};

/// Parsed MP4 `udta` atom.
/// GoPro cameras embed an undocumented
/// GPMF stream in the `udta` atom.
#[derive(Debug, Default)]
pub struct GoProMeta {
    pub path: PathBuf,
    pub udta: Vec<UdtaField>,
    pub gpmf: Vec<Stream>
}

impl GoProMeta {
    /// Extract custom GoPro metadata from MP4 `udta` atom.
    /// Mix of "normal" MP4 atom structures and GPMF-stream.
    pub fn new(path: &Path, debug: bool) -> Result<Self, GpmfError> {
        let mut mp4 = Mp4::new(path)?;
        let mut udta = mp4.udta()?;

        let mut meta = Self::default();
        meta.path = path.to_owned();

        // MP4 FourCC, not GPMF FourCC
        let fourcc_gpmf = FourCC::from_str("GPMF");

        for field in udta.fields.iter_mut() {
            if fourcc_gpmf == field.name {
                meta.gpmf.extend(Stream::new(&mut field.data, None, debug)?);
            } else {
                meta.udta.push(field.to_owned())
            }
        }

        Ok(meta)
    }

    pub fn muid(&self) -> Result<Vec<u32>, GpmfError> {
        let fourcc = FourCC::from_str("MUID");

        for field in self.udta.iter() {
            if field.name == fourcc {
                let no_of_entries = match ((field.size - 8) % 4, (field.size - 8) / 4) {
                    (0, n) => n,
                    (_, n) => panic!("Failed to determine MUID: {n} length field is not 32-bit aligned")
                };

                let mut fld = field.to_owned();

                return (0..no_of_entries).into_iter()
                        .map(|_| fld.data.read_le::<u32>()) // read LE to match GPMF
                        .collect::<BinResult<Vec<u32>>>()
                        .map_err(|err| GpmfError::BinReadError(err))
            }
        }

        Ok(Vec::new())
    }
}