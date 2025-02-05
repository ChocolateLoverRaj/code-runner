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
    println!("{}", env!("USER_SPACE"));

    #[cfg(debug_assertions)]
    {
        // create lldb debug files to make debugging easy
        let kernel_binary = env!("CARGO_BIN_FILE_KERNEL");
        let kernel_debug_file = "debug.lldb";
        std::fs::write(
            kernel_debug_file,
            [
                format!("target create {kernel_binary}"),
                format!("target modules load --file {kernel_binary} --slide 0xFFFF800000000000"),
                "gdb-remote localhost:1234".into(),
            ]
            .join("\n"),
        )
        .expect("unable to create debug file");

        let user_space_binary = env!("USER_SPACE");
        let user_space_debug_file = "debug_userspace.lldb";
        std::fs::write(
            user_space_debug_file,
            [
                format!("target create {user_space_binary}"),
                format!("target modules load --file {user_space_binary} --slide 0x0"),
                "gdb-remote localhost:1234".into(),
            ]
            .join("\n"),
        )
        .expect("unable to create debug file");

        println!("debug file is ready, run `lldb -s {}` to start debugging the kernel, or `lldb -s {}` to start debugging the user space program.", kernel_debug_file, user_space_debug_file);
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
