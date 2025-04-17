use limine::framebuffer::Framebuffer;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Clone, Copy)]
pub struct ScreenInfo {
    pub width: u64,
    pub height: u64,
    pub pitch: u64,
    pub bits_per_pixel: u16,
    pub red_mask_size: u8,
    pub red_mask_shift: u8,
    pub green_mask_size: u8,
    pub green_mask_shift: u8,
    pub blue_mask_size: u8,
    pub blue_mask_shift: u8,
}

impl From<&Framebuffer<'_>> for ScreenInfo {
    fn from(framebuffer: &Framebuffer) -> Self {
        ScreenInfo {
            width: framebuffer.width(),
            height: framebuffer.height(),
            pitch: framebuffer.pitch(),
            bits_per_pixel: framebuffer.bpp(),
            red_mask_size: framebuffer.red_mask_size(),
            red_mask_shift: framebuffer.red_mask_shift(),
            green_mask_size: framebuffer.green_mask_size(),
            green_mask_shift: framebuffer.green_mask_shift(),
            blue_mask_size: framebuffer.blue_mask_size(),
            blue_mask_shift: framebuffer.blue_mask_shift(),
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
#[non_exhaustive]
pub struct ScreenInfoWithAddress {
    pub address: usize,
    pub info: ScreenInfo,
}

impl ScreenInfoWithAddress {
    /// # Safety
    /// The address, together with size info, must point to a valid framebuffer
    pub unsafe fn new(address: usize, info: ScreenInfo) -> Self {
        Self { address, info }
    }
}

impl From<&Framebuffer<'_>> for ScreenInfoWithAddress {
    fn from(value: &Framebuffer<'_>) -> Self {
        Self {
            address: value.addr() as usize,
            info: value.into(),
        }
    }
}
