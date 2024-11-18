// src/bin/qemu-uefi.rs

use std::{
    env,
    process::{self, Command},
};

fn main() {
    let mut qemu = Command::new("qemu-system-x86_64");
    // qemu.arg("-m");
    // qemu.arg("4G");
    qemu.arg("-drive");
    println!("{}", env!("UEFI_IMAGE"));
    qemu.arg(format!("format=raw,file={}", env!("UEFI_IMAGE")));
    qemu.arg("-bios").arg(ovmf_prebuilt::ovmf_pure_efi());
    // qemu.arg("-serial").arg("stdio");
    let exit_status = qemu.status().unwrap();
    process::exit(exit_status.code().unwrap_or(-1));
}
