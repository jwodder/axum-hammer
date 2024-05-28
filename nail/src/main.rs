mod consts;
mod sleep;
mod subpages;
use crate::sleep::{sleep_for_params, sleep_handler, Sleeper};
use crate::subpages::SubpageService;
use anyhow::Context;
use axum::{
    extract::{Path, Request},
    routing::get,
    Router,
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
                    .with_target("tower_http", Level::TRACE)
                    .with_default(Level::INFO),
            )
            .init();
    }
    run(args)
}

#[tokio::main]
async fn run(args: Arguments) -> anyhow::Result<()> {
    let mut rng = thread_rng();
    let sleeper = Arc::new(Sleeper);
    let app = Router::new()
        .route("/hello", get(|| async { "Hello, world!\n" }))
        .route("/sleep", get(sleep_for_params))
        .nest("/subpages", {
            let subpages = SubpageService::new("/subpages", &mut rng);
            let subpages2 = subpages.clone();
            Router::new()
                .route("/", get(move || async move { subpages.index_response() }))
                .route(
                    "/:key",
                    get(move |key: Path<String>| async move { subpages2.subpage_response(&key) }),
                )
        })
        .nest("/subpages-arc", {
            let subpages = Arc::new(SubpageService::new("/subpages-arc", &mut rng));
            let subpages2 = Arc::clone(&subpages);
            Router::new()
                .route("/", get(move || async move { subpages.index_response() }))
                .route(
                    "/:key",
                    get(move |key: Path<String>| async move { subpages2.subpage_response(&key) }),
                )
        })
        .nest_service("/subpages-service", {
            let subpages = SubpageService::new("/subpages-service", &mut rng);
            service_fn(move |req: Request| {
                let s = subpages.clone();
                async move { s.handle_request(req).await }
            })
        })
        .nest_service("/subpages-service-arc", {
            let subpages = Arc::new(SubpageService::new("/subpages-service-arc", &mut rng));
            service_fn(move |req: Request| {
                let s = Arc::clone(&subpages);
                async move { s.handle_request(req).await }
            })
        })
        .nest_service("/sleep-service", service_fn(sleep_handler))
        .nest_service(
            "/sleep-arc-service",
            service_fn(move |req: Request| {
                let sleeper = Arc::clone(&sleeper);
                async move { sleeper.handle(req).await }
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
