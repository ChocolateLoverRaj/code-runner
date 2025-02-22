// build.rs

use bootloader::DiskImageBuilder;
use std::{env, path::PathBuf};

fn main() {
    let package_name = env::var("CARGO_PKG_NAME").unwrap();

    // set by cargo for the kernel artifact dependency
    let kernel_path = env::var("CARGO_BIN_FILE_KERNEL").unwrap();
    let mut disk_builder = DiskImageBuilder::new(PathBuf::from(&kernel_path));
    let userspace_path = env::var("CARGO_BIN_FILE_USER_SPACE").unwrap();
    disk_builder.set_ramdisk((&userspace_path).into());

    // specify output paths
    let out_dir = PathBuf::from(env::var("OUT_DIR").unwrap());
    let uefi_path = out_dir.join(format!("{package_name}-uefi.img"));
    let bios_path = out_dir.join(format!("{package_name}-bios.img"));

    // create the disk images
    disk_builder.create_uefi_image(&uefi_path).unwrap();
    disk_builder.create_bios_image(&bios_path).unwrap();

    // pass the disk image paths via environment variables
    println!("cargo:rustc-env=UEFI_IMAGE={}", uefi_path.display());
    println!("cargo:rustc-env=BIOS_IMAGE={}", bios_path.display());
    println!("cargo:rustc-env=CARGO_BIN_FILE_KERNEL={}", kernel_path);
    println!("cargo:rustc-env=USER_SPACE={}", userspace_path);
}
