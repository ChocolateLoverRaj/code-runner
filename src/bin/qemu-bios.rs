use code_runner::{run_qemu, BootType};

fn main() {
    run_qemu(BootType::Bios);
}
