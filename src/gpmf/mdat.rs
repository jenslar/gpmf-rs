//! Only concerns the initial GoPro "header" with device info at the very start of the `mdat` atom.
//!
//! Possible layout, following immediately after `mdat` FourCC (Max2 below):
//! - GPRO FourCC
//! - 32bit size (includes GPRO FourCC)
//! - 15 byte firmware version, e.g. H24.02.01.09.71 for Max2 same as udta GPMF section FMWR
//! - 16 byte lens ID (?) LEN1234567891011 for Max2 same as udta GPMF section LINF
//! - UP UNTIL HERE SAME ON MOST (ALL?) DEVICES
//!     - 32 bytes null padding: Fusion, Karma, Hero5, Hero6, Hero7, Hero10 (Karma is probably with Hero5 mounted)
//!     - 34 bytes null padding: Hero8
//!     - No padding: Max2, Hero12
//!     - Unknown: Hero9
//! - Try: read 16 bytes if all null:
//!     - YES: read 16 more assume these are null (or check for null again)
//!     - NO: interpret read bytes as 4 * u32 since this should be the first four MUID digits
//! - 1st MUID digit, 32bit LE (Hero5 is BE?)
//! - 2nd MUID digit, 32bit LE
//! - 3rd MUID digit, 32bit LE
//! - 4th MUID digit, 32bit LE
//! - 14 byte serial (?), null terminated? e.g. C3524224502446\0 (maybe no null, but null for other cameras too) for Max2 same as CASN in udta
//! - Dyncamically sized? 30 bytes? Device name, no obvious terminator
//! -

use binrw::BinRead;
use mp4iter::Mp4;

use crate::GpmfError;

/// GoPro header at the very start of the `mdat` atom,
/// immediately after `mdat` FourCC.
// #[derive(Debug, Default, BinRead)]
#[derive(Debug, Default)]
pub struct GoProMdatHeader {
    pub gpro: [u8; 4], // GPRO
    pub size: u32, // size incl GPRO 4CC, little endian
    pub firmware: [u8; 15], // string
    pub lens: [u8; 16], // string
    // add test: sum padding1, if zero read padding2,
    // otherwise skip padding2
    /// MUID first four digits OR null bytes/padding
    // #[br()]
    // padding1: [u8; 16],
    // /// null bytes/padding OR empty if previous padding were nulls
    // padding2: [u8; 16],
    // padding3: [u8; 2], // 34 bytes padding is only Hero8???
    // only read from file if padding1 is all zeros,
    // otherwise convert padding1 to [u32; 4]
    // #[br(map = |field| if padding1.iter().sum() == 0 {
    //     field
    // } else {
    //     padding1.chunks_exact(4).map(|c| )
    // } )]
    pub muid: [u32; 4],
    pub serial: [u8; 15], // null terminated string? C3501324855921\0
    // pub name: String, // 30 bytes incl null padding?
}

fn read_if<T>(condition: impl FnOnce() -> bool) -> impl FnOnce(T) -> Option<T> {
    move |field| condition().then(move || field)
}

impl GoProMdatHeader {
    /// Read a "GoPro header" that is located at the very start of the `mdat`
    /// byte load (unclear how/where this is used by GoPro) identified
    /// by the initial FourCC `GPRO` (note that FourCC, size order is reversed
    /// compared to MP4 structure: `GPRO` followed by section byte size as LE `u32`).
    pub fn read(mp4: &mut Mp4) -> Result<Self, GpmfError>{
        let mut atom = mp4.find_atom("mdat", true)?;
        let mut header = GoProMdatHeader::default();

        header.gpro = atom.read_one(binrw::Endian::Little, None)?; // 4 bytes
        header.size = atom.read_one(binrw::Endian::Little, None)?; // 4 bytes
        header.firmware = atom.read_one(binrw::Endian::Little, None)?; // 15 bytes
        header.lens = atom.read_one(binrw::Endian::Little, None)?; // 16 bytes

        let maybe_padding1: [u8; 16] = atom.read_one(binrw::Endian::Little, None)?;
        let padding1_is_null = maybe_padding1.iter().map(|n| *n as u16).sum::<u16>() == 0;
        if padding1_is_null {
            let maybe_padding2: [u8; 16] = atom.read_one(binrw::Endian::Little, None)?;
            let padding2_is_null = maybe_padding2.iter().map(|n| *n as u16).sum::<u16>() == 0;

        }

        Ok(header)
    }
}
