use runner::{BootType, run_qemu};

fn main() {
    run_qemu(BootType::Uefi);
}
