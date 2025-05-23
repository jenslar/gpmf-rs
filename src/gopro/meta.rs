//! GoPro MP4 metadata logged in the user data atom `udta`.
//!
//! GoPro embeds undocumented GPMF streams in the `udta` atom
//! that is also extracted.

use std::path::{Path, PathBuf};

use mp4iter::{FourCC, Mp4};

use crate::{Gpmf, GpmfError, GOPRO_UDTA_GPMF_FOURCC};

/// Representations MP4 `udta` atom
/// partially raw bytes, partially parsed
/// if a GPMF section is present (Hero 6 and later).
/// The embedded GPMF stream
/// is not further documented by GoPro,
/// but contains data such as firmware version.
#[derive(Debug, Default)]
pub struct GoProMeta {
    pub path: PathBuf,
    pub raw: Vec<(String, Vec<u8>)>,
    // pub muid: [u32; 8],
    // pub gumi: [u32; 4],
    // pub gpmf: Vec<Stream>
    pub gpmf: Gpmf
}

impl GoProMeta {
    /// Extract custom GoPro metadata from MP4 `udta` atom.
    /// Mix of "normal" MP4 atom structures and GPMF-data.
    pub fn new(path: &Path, debug: bool) -> Result<Self, GpmfError> {
        let mut mp4 = Mp4::new(path)?;

        let mut meta = Self::default();
        meta.path = path.to_owned();

        let udta_cursors = mp4.user_data_cursors()?;
        for (name, mut cursor) in udta_cursors.into_iter() {
            if name == GOPRO_UDTA_GPMF_FOURCC {
                meta.gpmf = Gpmf::from_cursor(&mut cursor, false)?;
            } else {
                meta.raw.push((name.to_string(), cursor.into_inner()))
            }
        }

        Ok(meta)
    }

    // fn muid() -> Result<Vec<u32>, GpmfError> {
    //     let fourcc = FourCC::from_str("MUID");

    //     // for field in self.udta.iter() {
    //     //     if field.name == fourcc {
    //     //         let no_of_entries = match ((field.size - 8) % 4, (field.size - 8) / 4) {
    //     //             (0, n) => n,
    //     //             (_, n) => panic!("Failed to determine MUID: {n} length field is not 32-bit aligned")
    //     //         };

    //     //         let mut fld = field.to_owned();

    //     //         return (0..no_of_entries).into_iter()
    //     //                 .map(|_| fld.data.read_le::<u32>()) // read LE to match GPMF
    //     //                 .collect::<BinResult<Vec<u32>>>()
    //     //                 .map_err(|err| GpmfError::BinReadError(err))
    //     //     }
    //     // }

    //     Ok(Vec::new())
    // }

    // fn gumi() {}
}
