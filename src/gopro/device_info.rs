use crate::{DeviceId, DeviceName, GpmfError, gopro::GoProMeta};

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct DeviceInfo {
    pub name: DeviceName,
    // id: DeviceId,
    pub firmware: String,
}

impl DeviceInfo {
    /// Reads device info from `udta` GPMF section.
    /// Note that the Hero5 Black has no GPMF data in the `udta` section.
    pub(crate) fn from_mp4(mp4: &mut mp4iter::Mp4) -> Result<Self, GpmfError> {
        let mut firm = mp4.find_user_data("FIRM")?;
        let device_string = firm.read_to_string()?;
        let name = DeviceName::from_firmware_id(&device_string);
        let version = &device_string[7..];

        Ok(Self {
            name,
            // id: DeviceId::default(),
            firmware: version.to_owned(),
        })
    }

    /// Reads device info from `udta` GPMF section.
    /// Note that the Hero5 Black has no GPMF data in the `udta` section.
    pub(crate) fn from_meta(meta: &GoProMeta) -> Result<Self, GpmfError> {
        meta.device()
    }

    pub fn name(&self) -> &str {
        self.name.to_str()
    }
}
