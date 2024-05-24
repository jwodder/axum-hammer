mod consts;
mod subpages;
mod types;
use crate::subpages::SubpageService;
use crate::types::SleepParams;
use anyhow::Context;
use axum::{
    extract::{Query, Request},
    response::IntoResponse,
    routing::get,
    RequestExt, Router,
};
use clap::Parser;
use rand::thread_rng;
use std::io::{stderr, IsTerminal};
use std::net::IpAddr;
use std::sync::Arc;
use tower::service_fn;
use tower_http::trace::TraceLayer;
use tracing::Level;
use tracing_subscriber::{filter::Targets, fmt::time::OffsetTime, prelude::*};

#[derive(Clone, Debug, Eq, Parser, PartialEq)]
struct Arguments {
    /// IP address to listen on
    #[arg(long, default_value = "127.0.0.1")]
    ip_addr: IpAddr,

    /// Port to listen on
    #[arg(short, long, default_value_t = 8080)]
    port: u16,

    #[arg(short = 'T', long)]
    trace: bool,
}

// See
// <https://docs.rs/tracing-subscriber/latest/tracing_subscriber/fmt/time/struct.OffsetTime.html#method.local_rfc_3339>
// for an explanation of the main + #[tokio::main]run thing
fn main() -> anyhow::Result<()> {
    let args = Arguments::parse();
    if args.trace {
        let timer =
            OffsetTime::local_rfc_3339().context("failed to determine local timezone offset")?;
        tracing_subscriber::registry()
            .with(
                tracing_subscriber::fmt::layer()
                    .with_timer(timer)
                    .with_ansi(stderr().is_terminal())
                    .with_writer(stderr),
            )
            .with(
                Targets::new()
                    .with_target(env!("CARGO_CRATE_NAME"), Level::TRACE)
                    .with_target("aws_config", Level::DEBUG)
                    .with_target("reqwest", Level::TRACE)
                    .with_target("reqwest_retry", Level::TRACE)
                    .with_target("tower_http", Level::TRACE)
                    .with_default(Level::INFO),
            )
            .init();
    }
    run(args)
}

#[tokio::main]
async fn run(args: Arguments) -> anyhow::Result<()> {
    let subpages = Arc::new(SubpageService::new(thread_rng()));
    let app = Router::new()
        .route("/hello", get(|| async { "Hello, world!\n" }))
        .route("/sleep", get(sleep_endpoint))
        .nest_service(
            "/subpages",
            service_fn(move |req: Request| {
                let subpages = Arc::clone(&subpages);
                async move { subpages.handle_request(req).await }
            }),
        )
        .nest_service(
            "/sleep-service",
            service_fn(|mut req: Request| async move {
                let r = match req.extract_parts::<Query<SleepParams>>().await {
                    Ok(params) => sleep_endpoint(params).await.into_response(),
                    Err(e) => e.into_response(),
                };
                Ok::<_, std::convert::Infallible>(r)
            }),
        )
        .layer(TraceLayer::new_for_http());
    let listener = tokio::net::TcpListener::bind((args.ip_addr, args.port))
        .await
        .context("failed to bind listener")?;
    axum::serve(listener, app)
        .await
        .context("failed to serve application")?;
    Ok(())
}

async fn sleep_endpoint(Query(params): Query<SleepParams>) -> String {
    let naptime = params.get_duration(thread_rng());
    tokio::time::sleep(naptime).await;
    format!("Slept for {naptime:?}\n")
}
