// build.rs

use bootloader::DiskImageBuilder;
use common::{permissions::Permissions, ram_disk::RamDisk};
use std::{borrow::Cow, env, fs, path::PathBuf};

fn main() {
    let package_name = env::var("CARGO_PKG_NAME").unwrap();

    // set by cargo for the kernel artifact dependency
    let kernel_path = env::var("CARGO_BIN_FILE_KERNEL").unwrap();
    let mut disk_builder = DiskImageBuilder::new(PathBuf::from(&kernel_path));
    let user_space_elf_path = env::var("CARGO_BIN_FILE_USER_SPACE").unwrap();

    // specify output paths
    let out_dir = PathBuf::from(env::var("OUT_DIR").unwrap());
    let uefi_path = out_dir.join(format!("{package_name}-uefi.img"));
    let bios_path = out_dir.join(format!("{package_name}-bios.img"));
    let ram_disk_path = out_dir.join("ram_disk");

    // Create the ram disk
    let ram_disk = RamDisk {
        permissions: Permissions {
            ports: Cow::Owned(
                {
                    let com1 = 0x3F8;
                    com1..com1 + 8
                }
                .into_iter()
                .collect(),
            ),
        },
        elf: Cow::Owned(fs::read(&user_space_elf_path).unwrap()),
    };
    fs::write(&ram_disk_path, postcard::to_allocvec(&ram_disk).unwrap()).unwrap();
    disk_builder.set_ramdisk(ram_disk_path);

    // create the disk images
    disk_builder.create_uefi_image(&uefi_path).unwrap();
    disk_builder.create_bios_image(&bios_path).unwrap();

    disk_builder
        .create_uefi_tftp_folder(&out_dir.join("folder"))
        .unwrap();

    // pass the disk image paths via environment variables
    println!("cargo:rustc-env=OUT_DIR={}", out_dir.display());
    println!("cargo:rustc-env=UEFI_IMAGE={}", uefi_path.display());
    println!("cargo:rustc-env=BIOS_IMAGE={}", bios_path.display());
    println!("cargo:rustc-env=CARGO_BIN_FILE_KERNEL={}", kernel_path);
    println!("cargo:rustc-env=USER_SPACE={}", user_space_elf_path);
}
