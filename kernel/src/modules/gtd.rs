use x86_64::instructions::tables::load_tss;
use x86_64::registers::segmentation::{Segment, CS, DS, ES, FS, GS, SS};
use x86_64::structures::tss::TaskStateSegment;

pub const DOUBLE_FAULT_IST_INDEX: u16 = 0;

use x86_64::structures::gdt::{Descriptor, GlobalDescriptorTable, SegmentSelector};

pub struct Gdt {
    gdt: GlobalDescriptorTable,
    code_selector: SegmentSelector,
    tss_selector: SegmentSelector,
}

impl Gdt {
    pub fn new(tss: &'static TaskStateSegment) -> Self {
        let mut gdt = GlobalDescriptorTable::new();
        let code_selector = gdt.append(Descriptor::kernel_code_segment());
        let tss_selector = gdt.append(Descriptor::tss_segment(&tss));
        Self {
            gdt,
            code_selector,
            tss_selector,
        }
    }

    pub fn init(&'static self) {
        self.gdt.load();
        unsafe {
            // https://github.com/rust-osdev/bootloader/blob/5d318bfc8afa4fb116a2c7923d5411febbe7266c/docs/migration/v0.9.md#kernel
            DS::set_reg(SegmentSelector::NULL);
            DS::set_reg(SegmentSelector::NULL);
            SS::set_reg(SegmentSelector::NULL);
            ES::set_reg(SegmentSelector::NULL);
            FS::set_reg(SegmentSelector::NULL);
            GS::set_reg(SegmentSelector::NULL);

            CS::set_reg(self.code_selector);
            load_tss(self.tss_selector);
        }
    }
}
