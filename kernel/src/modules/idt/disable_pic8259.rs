use pic8259::ChainedPics;

pub const PIC_1_OFFSET: u8 = 32;

pub fn disable_pic8259() {
    unsafe {
        let mut pics = ChainedPics::new_contiguous(PIC_1_OFFSET);
        pics.disable();
    }
}
