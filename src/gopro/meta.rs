//! GoPro MP4 metadata logged in the user data atom `udta`.
//!
//! GoPro embeds undocumented GPMF streams in the `udta` atom
//! that is also extracted.

use std::{collections::HashMap, io::{Cursor, Read}, path::{Path, PathBuf}};

use binrw::BinReaderExt;
use geojson::de;
use mp4iter::Mp4;

use crate::{DeviceInfo, DeviceName, FourCC::{self, MUID}, GOPRO_UDTA_GPMF_FOURCC, Gpmf, GpmfError, gpmf::Value};

/// Representations MP4 `udta` atom.
/// Partially raw bytes, partially parsed
/// if a GPMF section is present (Hero 6 and later).
/// The embedded GPMF stream
/// is not further documented by GoPro,
/// but contains data such as firmware version
/// settings and identifiers.
#[derive(Debug, Default, Clone, PartialEq)]
pub struct GoProMeta {
    path: PathBuf,
    raw: Vec<(String, Vec<u8>)>,
    gpmf: Gpmf
}

impl GoProMeta {
    /// Extract custom GoPro metadata from MP4 `udta` atom.
    /// Mix of "normal" MP4 atom structures and GPMF-data.
    pub fn new(path: &Path) -> Result<Self, GpmfError> {
        let mut mp4 = Mp4::new(path)?;

        Self::from_mp4(&mut mp4)
    }

    pub fn gpmf(&self) -> &Gpmf {
        &self.gpmf
    }

    pub fn iter_raw(&self) -> impl Iterator<Item = &(String, Vec<u8>)> {
        self.raw.iter()
    }

    pub(crate) fn from_mp4(mp4: &mut Mp4) -> Result<Self, GpmfError> {
        let mut meta = Self::default();
        meta.path = mp4.path().to_owned();

        let udta_cursors = mp4.user_data_cursors()?;
        for (name, mut cursor) in udta_cursors.into_iter() {
            if name == GOPRO_UDTA_GPMF_FOURCC {
                meta.gpmf = Gpmf::from_cursor(&mut cursor)?;
            } else {
                meta.raw.push((name.to_string(), cursor.into_inner()))
            }
        }

        Ok(meta)
    }

    /// Derive device info from FIRM section, either
    /// in raw udta section or udta GPMF section (Hero 6 and later)
    pub fn device(&self) -> Result<DeviceInfo, GpmfError> {
        let mut maybe_bytes: Option<Vec<u8>> = None;
        // All devices should have FIRM in normal udta section.
        if let Some((_, bytes)) = self.raw.iter().find(|(id, _)| id == &FourCC::FIRM.to_str()) {
            maybe_bytes = Some(bytes.to_owned())
        }
        // GUMI never existed in the GPMF section,
        // only in "normal" udta section.
        if let Some((_, bytes)) = self.raw.iter().find(|(id, _)| id == &FourCC::FIRM.to_str()) {
            maybe_bytes = Some(bytes.to_owned())
        }

        if let Some(bytes) = maybe_bytes {
            let mut rdr = Cursor::new(bytes);
            let mut device_string = String::new();
            // return rdr.read_be::<[u32; 4]>().unwrap_or_default()
            let _n = rdr.read_to_string(&mut device_string)?;
            let name = DeviceName::from_firmware_id(&device_string);
            let version = &device_string[7..];

            return Ok(DeviceInfo {
                name,
                // id: DeviceId::default(),
                firmware: version.to_owned(),
            })
        }

        return Err(GpmfError::NoDeviceId)
    }

    pub fn gumi(&self) -> [u32; 4] {
        // GUMI never existed in the GPMF section,
        // only in "normal" udta section.
        if let Some((_, bytes)) = self.raw.iter().find(|(id, _)| id == &FourCC::GUMI.to_str()) {
            let mut rdr = Cursor::new(bytes);
            return rdr.read_be::<[u32; 4]>().unwrap_or_default()
        }
        [0;4]
    }

    pub fn muid(&self) -> [u32; 8] {
        // Use MUID from "normal" udta section first.
        if let Some((_, bytes)) = self.raw.iter().find(|(id, _)| id == &FourCC::MUID.to_str()) {
            let mut rdr = Cursor::new(bytes);
            return rdr.read_be::<[u32; 8]>().unwrap_or_default()
        }

        // GPMF MUID is truncated to four digits for some models.
        if let Some(stream) = self.gpmf.find(&FourCC::MUID)
            && let Some(muid_vec) = stream.to_u32()
            && let Some(muid) = muid_vec.as_array::<8>() {
            return muid.to_owned()
        };
        [0;8]
    }

    /// Clip ID (session ID, despite its name).
    /// Only found in `udta` GPMF section.
    pub fn cpid(&self) -> Option<[u32; 4]>  {
        if let Some(stream) = self.gpmf.find(&FourCC::CPID)
            && let Some(cpid_vec) = stream.to_u32()
            && let Some(cpid) = cpid_vec.as_array::<4>(){
            return Some(cpid.to_owned())
        };
        None
    }

    /// Clip index. Order in recording session.
    /// Only found in `udta` GPMF section.
    pub fn cpin(&self) -> Option<usize>  {
        if let Some(stream) = self.gpmf.find(&FourCC::CPIN)
            && let Some(cpin_vec) = stream.to_u32()
            && let Some(cpin) = cpin_vec.first() {
            return Some(*cpin as usize)
        };
        None
    }
}
