use anyhow::Context;
use pic8259::ChainedPics;
use spin;

pub mod keyboard;
pub mod mouse;
pub mod timer;

pub const PIC_1_OFFSET: u8 = 32;

pub static PICS: spin::Mutex<ChainedPics> =
    spin::Mutex::new(unsafe { ChainedPics::new_contiguous(PIC_1_OFFSET) });

pub fn init() {
    let mut pics = PICS.lock();
    unsafe {
        pics.write_masks(0b11111000, 0b11101111);
        if let Err(e) = mouse::init().context("Error initializing mouse") {
            log::warn!("{:?}", e);
        };
        pics.initialize();
    };
}
