mod pcie;
mod sata;
mod usb;
mod wifi;

use crate::i18n::i18n;
use crate::utils::link::pcie::PcieLinkData;
use crate::utils::link::sata::SataSpeed;
use crate::utils::link::usb::UsbSpeed;
use crate::utils::link::wifi::WifiGeneration;
use anyhow::Result;
use std::fmt::{Display, Formatter};

#[derive(Debug, Default)]
pub enum Link {
    Pcie(LinkData<PcieLinkData>),
    Sata(LinkData<SataSpeed>),
    Usb(LinkData<UsbSpeed>),
    Wifi(LinkData<WifiGeneration>),
    #[default]
    Unknown,
}

#[derive(Debug)]
pub struct LinkData<T> {
    pub current: T,
    pub max: Result<T>,
}

impl<T> Display for LinkData<T>
where
    T: Display + PartialEq,
{
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let has_different_max = self.max.as_ref().map_or(false, |max| self.current != *max);
        if has_different_max {
            write!(f, "{} / {}", self.current, self.max.as_ref().unwrap())
        } else {
            write!(f, "{}", self.current)
        }
    }
}

impl Display for Link {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}",
            match self {
                Self::Pcie(data) => data.to_string(),
                Self::Sata(data) => data.to_string(),
                Self::Usb(data) => data.to_string(),
                Self::Wifi(data) => data.to_string(),
                Self::Unknown => i18n("N/A"),
            }
        )
    }
}
