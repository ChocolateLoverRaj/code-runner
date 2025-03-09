use std::net::{IpAddr, Ipv4Addr};

use rocket::{fs::FileServer, get, http::uri::Host, routes, Config};

use crate::PORT;

#[get("/boot.ipxe")]
fn index(host: &Host) -> String {
    format!(
        r#"#!ipxe
module http://{host}/kernel-x86_64
module http://{host}/ramdisk
chain http://{host}/bootloader
"#
    )
}

pub async fn run_server() {
    let out_dir = env!("OUT_DIR");

    rocket::custom(Config {
        port: PORT,
        address: IpAddr::V4(Ipv4Addr::UNSPECIFIED),
        // tls: Some(
        //     TlsConfig::from_paths(
        //         PathBuf::from_iter([FOLDER, CERT_FILE]),
        //         PathBuf::from_iter([FOLDER, SERVER_KEY]),
        //     )
        // ),
        ..Default::default()
    })
    .mount("/", routes![index])
    .mount("/", FileServer::from(format!("{}/folder", out_dir)))
    .launch()
    .await
    .unwrap();
}
