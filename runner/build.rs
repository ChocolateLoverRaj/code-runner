// build.rs

use common::{
    permissions::{MetaData, Permissions},
    ram_disk::RamDisk,
};
use std::{
    borrow::Cow,
    env,
    fs::{self, create_dir_all, remove_file},
    io::{self, ErrorKind},
    os::unix::fs::symlink,
    path::{Path, PathBuf},
    process::Stdio,
};

fn main() {
    let out_dir = PathBuf::from(env::var("OUT_DIR").unwrap());
    let output_iso = out_dir.join("code_runner.iso");
    let iso_dir = out_dir.join("iso_root");
    create_dir_all(&iso_dir).unwrap();
    let boot_dir = iso_dir.join("boot");
    create_dir_all(&boot_dir).unwrap();
    let limine_dir = boot_dir.join("limine");
    create_dir_all(&limine_dir).unwrap();
    let efi_boot_dir = iso_dir.join("EFI/BOOT");
    create_dir_all(&efi_boot_dir).unwrap();

    let limine_conf = limine_dir.join("limine.conf");
    let runner_dir = PathBuf::from(std::env::var("CARGO_MANIFEST_DIR").unwrap());
    ensure_symlink(runner_dir.join("limine.conf"), limine_conf).unwrap();

    let limine_build = runner_dir.parent().unwrap().join("limine");

    for path in [
        "limine-bios.sys",
        "limine-bios-cd.bin",
        "limine-uefi-cd.bin",
    ] {
        let from = limine_build.join("share/limine").join(path);
        let to = limine_dir.join(path);
        ensure_symlink(from, to).unwrap();
    }

    for path in ["BOOTX64.EFI", "BOOTIA32.EFI"] {
        let from = limine_build.join("share/limine").join(path);
        let to = efi_boot_dir.join(path);
        ensure_symlink(from, to).unwrap();
    }

    let kernel_src = env::var("CARGO_BIN_FILE_KERNEL").unwrap();
    let kernel_dest = boot_dir.join("kernel");
    ensure_symlink(&kernel_src, &kernel_dest).unwrap();

    let ram_disk_path = boot_dir.join("ram_disk");

    // Create the ram disk
    let user_space_elf_path = env::var("CARGO_BIN_FILE_USER_SPACE").unwrap();
    let ram_disk = RamDisk {
        meta_data: MetaData {
            stack_size: 0x4000,
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
        },
        elf: Cow::Owned(fs::read(&user_space_elf_path).unwrap()),
    };
    fs::write(&ram_disk_path, postcard::to_allocvec(&ram_disk).unwrap()).unwrap();

    let status = std::process::Command::new("xorriso")
        .arg("-as")
        .arg("mkisofs")
        .arg("--follow-links")
        .arg("-b")
        .arg(
            limine_dir
                .join("limine-bios-cd.bin")
                .strip_prefix(&iso_dir)
                .unwrap(),
        )
        .arg("-no-emul-boot")
        .arg("-boot-load-size")
        .arg("4")
        .arg("-boot-info-table")
        .arg("--efi-boot")
        .arg(
            limine_dir
                .join("limine-uefi-cd.bin")
                .strip_prefix(&iso_dir)
                .unwrap(),
        )
        .arg("-efi-boot-part")
        .arg("--efi-boot-image")
        .arg("--protective-msdos-label")
        .arg(iso_dir)
        .arg("-o")
        .arg(&output_iso)
        .stderr(Stdio::inherit())
        .stdout(Stdio::inherit())
        .status()
        .unwrap();
    assert!(status.success());

    let status = std::process::Command::new(limine_build.join("bin/limine"))
        .arg("bios-install")
        .arg(&output_iso)
        .stderr(Stdio::inherit())
        .stdout(Stdio::inherit())
        .status()
        .unwrap();
    assert!(status.success());

    let chain_loader_efi_path = env::var("CARGO_BIN_FILE_CHAIN_LOADER").unwrap();

    // pass the disk image paths via environment variables
    println!("cargo:rustc-env=OUT_DIR={}", out_dir.display());
    println!("cargo:rustc-env=ISO={}", output_iso.display());
    println!("cargo:rustc-env=CARGO_BIN_FILE_KERNEL={}", kernel_src);
    println!("cargo:rustc-env=USER_SPACE={}", user_space_elf_path);
    println!("cargo:rustc-env=CHAIN_LOADER={}", chain_loader_efi_path);
}

pub fn ensure_symlink<P: AsRef<Path>, Q: AsRef<Path>>(original: P, link: Q) -> io::Result<()> {
    match remove_file(&link) {
        Ok(()) => Ok(()),
        Err(error) => match error.kind() {
            ErrorKind::NotFound => Ok(()),
            _ => Err(error),
        },
    }?;
    symlink(original, link)?;
    Ok(())
}
