use std::net::{IpAddr, Ipv4Addr};

use rocket::{Config, fs::FileServer, get, http::uri::Host, routes};

use crate::PORT;

#[get("/boot.ipxe")]
fn index(host: &Host) -> String {
    format!(
        r#"#!ipxe
chain http://{host}/chain_loader.efi
"#
    )
}

pub async fn run_server() {
    let out_dir = env!("OUT_DIR");
    let chain_loader = env!("CHAIN_LOADER");

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
    .mount(
        "/chain_loader.efi",
        FileServer::new(chain_loader, rocket::fs::Options::IndexFile),
    )
    // .mount("/", FileServer::from(out_dir))
    .launch()
    .await
    .unwrap();
}
