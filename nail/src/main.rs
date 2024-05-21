use anyhow::Context;
use axum::{extract::Query, routing::get, Router};
use clap::Parser;
use rand::{thread_rng, Rng};
use serde::Deserialize;
use std::net::IpAddr;
use std::time::Duration;
use thiserror::Error;

#[derive(Clone, Debug, Eq, Parser, PartialEq)]
struct Arguments {
    /// IP address to listen on
    #[arg(long, default_value = "127.0.0.1")]
    ip_addr: IpAddr,

    /// Port to listen on
    #[arg(short, long, default_value_t = 8080)]
    port: u16,
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

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let args = Arguments::parse();
    let app = Router::new()
        .route("/hello", get(|| async { "Hello, world!\n" }))
        .route("/sleep", get(sleep_endpoint));
    let listener = tokio::net::TcpListener::bind((args.ip_addr, args.port))
        .await
        .context("failed to bind listener")?;
    axum::serve(listener, app)
        .await
        .context("failed to serve application")?;
    Ok(())
}

async fn sleep_endpoint(Query(params): Query<SleepParams>) -> String {
    let naptime = params.get_duration(&mut thread_rng());
    tokio::time::sleep(naptime).await;
    format!("Slept for {naptime:?}\n")
}
