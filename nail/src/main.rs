use anyhow::Context;
use axum::{extract::Query, routing::get, Router};
use clap::Parser;
use rand::{thread_rng, Rng};
use serde::Deserialize;
use std::io::{stderr, IsTerminal};
use std::net::IpAddr;
use std::time::Duration;
use thiserror::Error;
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

#[derive(Copy, Clone, Debug, Deserialize, Eq, PartialEq)]
#[serde(try_from = "RawSleepParams")]
struct SleepParams {
    min: u64,
    max: u64,
}

impl SleepParams {
    fn get_duration<R: Rng>(&self, mut rng: R) -> Duration {
        Duration::from_millis(rng.gen_range(self.min..=self.max))
    }
}

impl TryFrom<RawSleepParams> for SleepParams {
    type Error = SleepParamsError;

    fn try_from(value: RawSleepParams) -> Result<SleepParams, SleepParamsError> {
        let min = value.min.unwrap_or(500);
        let max = match value.max {
            Some(max) if min < max => max,
            Some(_) => return Err(SleepParamsError),
            None => min.saturating_mul(2),
        };
        Ok(SleepParams { min, max })
    }
}

#[derive(Copy, Clone, Debug, Eq, Error, PartialEq)]
#[error("min must be greater than max")]
struct SleepParamsError;

#[derive(Copy, Clone, Debug, Deserialize, Eq, PartialEq)]
struct RawSleepParams {
    min: Option<u64>,
    max: Option<u64>,
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
    let app = Router::new()
        .route("/hello", get(|| async { "Hello, world!\n" }))
        .route("/sleep", get(sleep_endpoint))
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
