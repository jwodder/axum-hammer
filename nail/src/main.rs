use anyhow::Context;
use axum::{routing::get, Router};
use clap::Parser;
use std::net::IpAddr;

#[derive(Clone, Debug, Eq, Parser, PartialEq)]
struct Arguments {
    /// IP address to listen on
    #[arg(long, default_value = "127.0.0.1")]
    ip_addr: IpAddr,

    /// Port to listen on
    #[arg(short, long, default_value_t = 8080)]
    port: u16,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let args = Arguments::parse();
    let app = Router::new().route("/hello", get(|| async { "Hello, world!\n" }));
    let listener = tokio::net::TcpListener::bind((args.ip_addr, args.port))
        .await
        .context("failed to bind listener")?;
    axum::serve(listener, app)
        .await
        .context("failed to serve application")?;
    Ok(())
}
