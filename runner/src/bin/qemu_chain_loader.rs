use std::{env, fs, path::PathBuf, process::Command};

fn main() {
    let chain_loader_path = env!("CHAIN_LOADER");
    let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let esp_path = dir.parent().unwrap().join("esp");

    println!("{}", chain_loader_path);

    println!("Current directory: {:?}", dir);

    fs::copy(
        chain_loader_path,
        esp_path.join("efi").join("boot").join("bootx64.efi"),
    )
    .unwrap();

    // qemu-system-x86_64 -enable-kvm -drive if=pflash,format=raw,readonly=on,file=$OVMF_PATH -drive format=raw,file=fat:rw:esp
    let mut command = Command::new("qemu-system-x86_64");
    command
        .arg("-enable-kvm")
        .arg("-drive")
        .arg(format!(
            "if=pflash,format=raw,readonly=on,file={}",
            env!("OVMF_PATH")
        ))
        .arg("-drive")
        .arg(format!(
            "format=raw,file=fat:rw:{}",
            esp_path.to_str().unwrap()
        ));
    env::args().skip(1).for_each(|arg| {
        command.arg(arg);
    });
    command.status().unwrap();
}
