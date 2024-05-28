mod buffered_tasks;
use crate::buffered_tasks::BufferedTasks;
use clap::{Parser, Subcommand};
use futures_util::TryStreamExt;
use statrs::statistics::{Data, Distribution};
use std::num::NonZeroUsize;
use std::time::{Duration, Instant};
use url::Url;

#[derive(Clone, Debug, Eq, Parser, PartialEq)]
struct Arguments {
    #[command(subcommand)]
    command: Command,
}

#[derive(Clone, Debug, Eq, PartialEq, Subcommand)]
enum Command {
    Run {
        url: Url,

        requests: NonZeroUsize,

        #[arg(required = true)]
        workers: Vec<NonZeroUsize>,
    },
    Subpages {
        url: Url,

        #[arg(required = true)]
        workers: Vec<NonZeroUsize>,
    },
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let (session, worker_qtys) = match Arguments::parse().command {
        Command::Run {
            url,
            requests,
            workers,
        } => (Session::Repeat { url, requests }, workers),
        Command::Subpages { url, workers } => (Session::Subpages { root_url: url }, workers),
    };
    println!("{}", Report::csv_header());
    for w in worker_qtys {
        let r = session.run(w).await?;
        println!("{}", r.as_csv());
    }
    Ok(())
}

#[derive(Clone, Debug, Eq, PartialEq)]
enum Session {
    Repeat { url: Url, requests: NonZeroUsize },
    Subpages { root_url: Url },
}

impl Session {
    async fn run(&self, workers: NonZeroUsize) -> anyhow::Result<Report> {
        let (times, overall_time) = match self {
            Session::Repeat { url, requests } => {
                let start = Instant::now();
                let tasks = BufferedTasks::from_iter(
                    workers.get(),
                    std::iter::repeat(url.clone())
                        .take(requests.get())
                        .map(get_url),
                );
                let times = tasks.try_collect::<Vec<_>>().await?;
                (times, start.elapsed())
            }
            Self::Subpages { root_url } => {
                let start = Instant::now();
                let r = reqwest::get(root_url.clone()).await?.error_for_status()?;
                let body = r.text().await?;
                let mut times = vec![start.elapsed()];
                let mut tasks = BufferedTasks::from_iter(
                    workers.get(),
                    body.lines().map(|path| {
                        get_url(
                            root_url
                                .join(path)
                                .expect("URL should be able to be a base"),
                        )
                    }),
                );
                while let Some(d) = tasks.try_next().await? {
                    times.push(d);
                }
                (times, start.elapsed())
            }
        };
        let Stats { mean, stddev, qty } = Stats::for_durations(&times);
        Ok(Report {
            workers,
            requests: qty,
            request_time_mean: mean,
            request_time_stddev: stddev,
            overall_time,
        })
    }
}

#[derive(Clone, Debug, PartialEq)]
struct Report {
    workers: NonZeroUsize,
    requests: usize,
    request_time_mean: f64,
    request_time_stddev: f64,
    overall_time: Duration,
}

impl Report {
    fn csv_header() -> &'static str {
        "workers,requests,request_time_mean,request_time_stddev,overall_time"
    }

    fn as_csv(&self) -> String {
        format!(
            "{},{},{},{},{}",
            self.workers,
            self.requests,
            self.request_time_mean,
            self.request_time_stddev,
            self.overall_time.as_secs_f64()
        )
    }
}

#[derive(Clone, Debug, PartialEq)]
struct Stats {
    mean: f64,
    stddev: f64,
    qty: usize,
}

impl Stats {
    fn for_durations(durations: &[Duration]) -> Stats {
        let times = durations
            .iter()
            .map(Duration::as_secs_f64)
            .collect::<Vec<_>>();
        let data = Data::new(times);
        let mean = data
            .mean()
            .expect("mean should exist for nonzero number of samples");
        let stddev = data
            .std_dev()
            .expect("stddev should exist for nonzero number of samples");
        let qty = data.len();
        Stats { mean, stddev, qty }
    }
}

async fn get_url(url: Url) -> anyhow::Result<Duration> {
    let start = Instant::now();
    let r = reqwest::get(url).await?.error_for_status()?;
    let _ = r.bytes().await?;
    Ok(start.elapsed())
}
