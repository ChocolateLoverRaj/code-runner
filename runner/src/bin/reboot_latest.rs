use rocket::tokio::{join, task::spawn_blocking};
use runner::server::run_server;
use serialport::SerialPortType;

/// Copies UEFI image to the target Chromebook, and make the target Chromebook reboot into it
#[rocket::main]
async fn main() {
    let server_future = run_server();
    let ec_future = spawn_blocking(|| {
        // Reboot the Chromebook
        let ec_console = serialport::available_ports()
            .unwrap()
            .into_iter()
            .find(|port| match &port.port_type {
                // Cr50 connected through USB
                SerialPortType::UsbPort(usb_port_info) => {
                    // /dev/ttyUSB2 is the EC console
                    usb_port_info.vid == 0x18d1
                        && usb_port_info.pid == 0x5014
                        && usb_port_info.interface == Some(2)
                }
                _ => false,
            })
            .unwrap();
        let mut ec_console = serialport::new(ec_console.port_name, 115200)
            .open()
            .unwrap();
        ec_console.write_all(b"apreset\n").unwrap();
    });

    let (_, result) = join!(server_future, ec_future);
    result.unwrap()
}
