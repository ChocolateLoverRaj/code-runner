use std::{fs, path::Path, process::Command};

// Copies the UEFI image to a folder and unmounts the disk
fn main() {
    let src = Path::new(env!("UEFI_IMAGE"));
    let file_name = src.file_name().unwrap().to_str().unwrap();
    let dest_folder = "/run/media/rajas/Ventoy";
    let dest = format!("{}/{}", dest_folder, file_name);
    fs::copy(src, &dest).unwrap();
    println!("Copied {:?} to {:?}", src, dest);

    Command::new("umount").arg(dest_folder).output().unwrap();
    println!("Unmounted the drive. Ready to unplug and boot the OS.");
}
