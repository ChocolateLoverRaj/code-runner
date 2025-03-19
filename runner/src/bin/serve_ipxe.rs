use runner::server::run_server;

#[rocket::main]
async fn main() {
    run_server().await;
}
