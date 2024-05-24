use axum::{
    body::Body,
    extract::{Query, Request},
    http::response::Response,
    response::IntoResponse,
    RequestExt,
};
use rand::thread_rng;
use rand::Rng;
use serde::Deserialize;
use std::convert::Infallible;
use std::time::Duration;
use thiserror::Error;

#[derive(Copy, Clone, Debug, Deserialize, Eq, PartialEq)]
#[serde(try_from = "RawSleepParams")]
pub(crate) struct SleepParams {
    min: u64,
    max: u64,
}

impl SleepParams {
    pub(crate) fn get_duration<R: Rng>(&self, mut rng: R) -> Duration {
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

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct Sleeper;

impl Sleeper {
    pub(crate) async fn handle(&self, req: Request) -> Result<Response<Body>, Infallible> {
        sleep_handler(req).await
    }
}

pub(crate) async fn sleep_handler(mut req: Request) -> Result<Response<Body>, Infallible> {
    match req.extract_parts::<Query<SleepParams>>().await {
        Ok(params) => Ok(sleep_for_params(params).await),
        Err(e) => Ok(e.into_response()),
    }
}

pub(crate) async fn sleep_for_params(Query(params): Query<SleepParams>) -> Response<Body> {
    let naptime = params.get_duration(thread_rng());
    tokio::time::sleep(naptime).await;
    format!("Slept for {naptime:?}\n").into_response()
}
