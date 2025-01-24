// src/bin/qemu-bios.rs

use std::{
    env,
    process::{self, Command},
};

fn main() {
    println!("{}", env!("BIOS_IMAGE"));
    println!("{}", env!("CARGO_BIN_FILE_KERNEL"));
    let mut qemu = Command::new("qemu-system-x86_64");
    qemu.arg("-drive");
    qemu.arg(format!("format=raw,file={}", env!("BIOS_IMAGE")));
    env::args().skip(1).for_each(|arg| {
        qemu.arg(arg);
    });
    let exit_status = qemu.status().unwrap();
    process::exit(exit_status.code().unwrap_or(-1));
}
