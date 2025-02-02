use std::{
    env,
    process::{self, Command},
};

pub enum BootType {
    Bios,
    Uefi,
}

pub fn run_qemu(boot_type: BootType) {
    println!("{}", env!("UEFI_IMAGE"));
    println!("{}", env!("CARGO_BIN_FILE_KERNEL"));
    println!("{}", env!("USERSPACE"));

    #[cfg(debug_assertions)]
    {
        let kernel_binary = env!("CARGO_BIN_FILE_KERNEL");
        // create an lldb debug file to make debugging easy
        let content = format!(
            r#"target create {kernel_binary}
                target modules load --file {kernel_binary} --slide 0xFFFF800000000000
                gdb-remote localhost:1234"#
        );
        std::fs::write("debug.lldb", content).expect("unable to create debug file");
        println!("debug file is ready, run `lldb -s debug.lldb` to start debugging");
    }

    let mut qemu = Command::new("qemu-system-x86_64");
    // To increase memory
    // qemu.arg("-m");
    // qemu.arg("4G");
    // To show serial port in terminal
    // qemu.arg("-serial").arg("stdio");
    // To enable debugging and pause
    // qemu.arg("-s").arg("-S");
    qemu.arg("-drive");
    qemu.arg(format!(
        "format=raw,file={}",
        match boot_type {
            BootType::Bios => {
                env!("BIOS_IMAGE")
            }
            BootType::Uefi => {
                env!("UEFI_IMAGE")
            }
        }
    ));
    if let BootType::Uefi = boot_type {
        qemu.arg("-bios").arg(ovmf_prebuilt::ovmf_pure_efi());
    }
    env::args().skip(1).for_each(|arg| {
        qemu.arg(arg);
    });
    let exit_status = qemu.status().unwrap();
    process::exit(exit_status.code().unwrap_or(-1));
}
