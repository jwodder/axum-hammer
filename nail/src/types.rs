use rand::Rng;
use serde::Deserialize;
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
