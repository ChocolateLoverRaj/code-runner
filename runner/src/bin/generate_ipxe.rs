use std::{
    fs::{create_dir_all, write},
    path::PathBuf,
};

use gethostname::gethostname;
use rcgen::{
    BasicConstraints, CertificateParams, DistinguishedName, DnType, IsCa, KeyPair, PKCS_RSA_SHA256,
};
use runner::{CERT_FILE, FOLDER, PORT, SERVER_KEY};

/// Generate an iPXE script
fn main() {
    let host_name = gethostname();
    let host_name = host_name.to_str().unwrap();

    create_dir_all(FOLDER).unwrap();

    // Don't do HTTPS cuz iPXE doesn't support the latest TLS certificate algorithms
    // let mut certificate_params = CertificateParams::new([host_name.to_owned()]).unwrap();
    // certificate_params.is_ca = IsCa::Ca(BasicConstraints::Unconstrained);
    // certificate_params.distinguished_name = {
    //     let mut distinguished_name = DistinguishedName::default();
    //     distinguished_name.push(
    //         DnType::CommonName,
    //         &format!("Code Runner Server ({})", host_name),
    //     );
    //     distinguished_name
    // };
    // let key_pair = KeyPair::generate_for(&PKCS_RSA_SHA256).unwrap();
    // let certificate = certificate_params.self_signed(&key_pair).unwrap();

    // write(PathBuf::from_iter([FOLDER, CERT_FILE]), certificate.pem()).unwrap();
    // write(
    //     PathBuf::from_iter([FOLDER, SERVER_KEY]),
    //     key_pair.serialize_pem(),
    // )
    // .unwrap();

    let ipxe_file = "http_boot.ipxe";
    write(
        PathBuf::from_iter([FOLDER, ipxe_file]),
        format!(
            r#"#!ipxe
dhcp
chain --replace http://{}:{}/boot.ipxe
"#,
            host_name, PORT
        ),
    )
    .unwrap();

    println!(
        "Generated an iPXE script and a HTTPS certificate. Embed the script by specifying EMBED=./{}/{} and embed the certificate by specifying TRUST=./{}/{} when building iPXE. Or use the ./build_ipxe.sh if you have Nix installed.",
        FOLDER, ipxe_file, FOLDER, CERT_FILE
    );
}
