use core::fmt::Debug;

use limine::request::RsdpRequest;
use thiserror::Error;

/// A wrapper around u64 that represents the physical address of the RSDP structure.
#[derive(Clone, Copy)]
pub struct RsdpAddr(u64);

impl Debug for RsdpAddr {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "RsdpAddr(0x{:X})", self.0)
    }
}

#[derive(Debug, Error)]
pub enum FromRsdpRequestError {
    #[error("Response was None")]
    NoResponse,
}

impl TryFrom<&RsdpRequest> for RsdpAddr {
    type Error = FromRsdpRequestError;

    fn try_from(value: &RsdpRequest) -> Result<Self, Self::Error> {
        Ok(Self(
            value
                .get_response()
                .ok_or(FromRsdpRequestError::NoResponse)?
                .address() as u64,
        ))
    }
}

impl From<RsdpAddr> for u64 {
    fn from(value: RsdpAddr) -> Self {
        value.0
    }
}
