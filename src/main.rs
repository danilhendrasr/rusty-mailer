use std::net::TcpListener;

use zero2prod::run;

#[tokio::main]
async fn main() -> std::io::Result<()> {
    let url_to_bind = "http://127.0.0.1:8081";
    let listener = TcpListener::bind(url_to_bind).expect("Failed binding to port");
    run(listener)?.await
}
