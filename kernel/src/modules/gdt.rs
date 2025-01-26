use log::info;
use x86_64::instructions::tables::load_tss;
use x86_64::registers::segmentation::{Segment, CS, DS, ES, FS, GS, SS};
use x86_64::structures::tss::TaskStateSegment;

pub const DOUBLE_FAULT_IST_INDEX: u16 = 0;

use x86_64::structures::gdt::{
    Descriptor, DescriptorFlags, GlobalDescriptorTable, SegmentSelector,
};

pub struct Gdt {
    gdt: GlobalDescriptorTable,
    kernel_code_selector: SegmentSelector,
    kernel_data_selector: SegmentSelector,
    pub user_code_selector: SegmentSelector,
    pub user_data_selector: SegmentSelector,
    tss_selector: SegmentSelector,
}

impl Gdt {
    pub fn new(tss: &'static TaskStateSegment) -> Self {
        let mut gdt = GlobalDescriptorTable::new();
        let kernel_code_selector = gdt.append(Descriptor::kernel_code_segment());
        let kernel_data_selector = gdt.append(Descriptor::kernel_data_segment());
        let tss_selector = gdt.append(Descriptor::tss_segment(&tss));
        let user_code_selector = gdt.append(Descriptor::user_code_segment());
        let user_data_selector = gdt.append(Descriptor::user_data_segment());

        info!(
            "kernel code: {:?}\nkernel data: {:?}\nuser code: {:?}\nuser data: {:?}\nTSS: {:?}",
            kernel_code_selector,
            kernel_data_selector,
            user_code_selector,
            user_data_selector,
            tss_selector
        );

        Self {
            gdt,
            kernel_code_selector,
            kernel_data_selector,
            user_code_selector,
            user_data_selector,
            tss_selector,
        }
    }

    pub fn init(&self) {
        unsafe { self.gdt.load_unsafe() };
        unsafe {
            // https://github.com/rust-osdev/bootloader/blob/5d318bfc8afa4fb116a2c7923d5411febbe7266c/docs/migration/v0.9.md#kernel
            SS::set_reg(SegmentSelector::NULL);
            ES::set_reg(SegmentSelector::NULL);
            FS::set_reg(SegmentSelector::NULL);
            GS::set_reg(SegmentSelector::NULL);

            CS::set_reg(self.kernel_code_selector);
            DS::set_reg(self.kernel_data_selector);
            load_tss(self.tss_selector);
        }
    }
}
