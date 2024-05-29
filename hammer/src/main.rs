mod show_duration;
mod tasks;
use crate::show_duration::show_duration_as_seconds;
use crate::tasks::request_tasks;
use anyhow::Context;
use clap::{Parser, Subcommand};
use futures_util::TryStreamExt;
use serde::Serialize;
use statrs::statistics::{Data, Distribution};
use std::fs::File;
use std::io::{BufWriter, Write};
use std::num::NonZeroUsize;
use std::path::PathBuf;
use std::time::{Duration, Instant};
use time::OffsetDateTime;
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
        #[arg(short = 'J', long)]
        json_file: Option<PathBuf>,

        #[arg(short, long, default_value = "10")]
        samples: NonZeroUsize,

        url: Url,

        #[arg(required = true)]
        workers: Vec<NonZeroUsize>,
    },
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let (mut reporter, session, worker_qtys) = match Arguments::parse().command {
        Command::Run {
            url,
            requests,
            workers,
        } => (Reporter::csv(), Session::Repeat { url, requests }, workers),
        Command::Subpages {
            json_file,
            url,
            workers,
            samples,
        } => (
            if let Some(path) = json_file {
                Reporter::json(path, url.clone())
            } else {
                Reporter::csv()
            },
            Session::Subpages { root_url: url },
            workers
                .into_iter()
                .flat_map(|w| std::iter::repeat(w).take(samples.get()))
                .collect(),
        ),
    };
    let client = reqwest::Client::builder()
        .build()
        .context("failed to create client")?;
    reporter.start();
    for w in worker_qtys {
        let r = session.run(&client, w).await?;
        reporter.process(r);
    }
    reporter.end()?;
    Ok(())
}

#[derive(Clone, Debug, Eq, PartialEq)]
enum Session {
    Repeat { url: Url, requests: NonZeroUsize },
    Subpages { root_url: Url },
}

impl Session {
    async fn run(&self, client: &reqwest::Client, workers: NonZeroUsize) -> anyhow::Result<Report> {
        let (request_times, overall_time) = match self {
            Session::Repeat { url, requests } => {
                let start = Instant::now();
                let tasks = request_tasks(
                    client,
                    workers.get(),
                    std::iter::repeat(url.clone()).take(requests.get()),
                );
                let times = tasks.try_collect::<Vec<_>>().await?;
                (times, start.elapsed())
            }
            Self::Subpages { root_url } => {
                let start = Instant::now();
                let r = client
                    .get(root_url.clone())
                    .send()
                    .await?
                    .error_for_status()?;
                let body = r.text().await?;
                let mut times = vec![start.elapsed()];
                let mut tasks = request_tasks(
                    client,
                    workers.get(),
                    body.lines().map(|path| {
                        root_url
                            .join(path)
                            .expect("URL should be able to be a base")
                    }),
                );
                while let Some(d) = tasks.try_next().await? {
                    times.push(d);
                }
                (times, start.elapsed())
            }
        };
        Ok(Report {
            workers,
            request_times,
            overall_time,
        })
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
struct Report {
    workers: NonZeroUsize,
    request_times: Vec<Duration>,
    overall_time: Duration,
}

impl Report {
    fn csv_header() -> &'static str {
        "workers,requests,request_time_mean,request_time_stddev,overall_time"
    }

    fn as_csv(&self) -> String {
        let Stats { mean, stddev, qty } = Stats::for_durations(&self.request_times);
        format!(
            "{workers},{qty},{mean},{stddev},{elapsed}",
            workers = self.workers,
            elapsed = show_duration_as_seconds(self.overall_time),
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

#[derive(Clone, Debug, Eq, PartialEq)]
enum Reporter {
    Json { outfile: PathBuf, data: StatReport },
    Csv,
}

impl Reporter {
    fn json(outfile: PathBuf, base_url: Url) -> Self {
        Reporter::Json {
            outfile,
            data: StatReport::new(base_url),
        }
    }

    fn csv() -> Self {
        Reporter::Csv
    }

    fn start(&mut self) {
        match self {
            Reporter::Json { data, .. } => data.start_time = Some(OffsetDateTime::now_utc()),
            Reporter::Csv => println!("{}", Report::csv_header()),
        }
    }

    fn process(&mut self, report: Report) {
        match self {
            Reporter::Json { data, .. } => {
                eprintln!(
                    "Finished: workers = {}, requests = {}, elapsed = {:?}",
                    report.workers,
                    report.request_times.len(),
                    report.overall_time
                );
                data.traversals.push(report);
            }
            Reporter::Csv => println!("{}", report.as_csv()),
        }
    }

    fn end(self) -> anyhow::Result<()> {
        match self {
            Reporter::Json { outfile, mut data } => {
                data.end_time = Some(OffsetDateTime::now_utc());
                let mut fp =
                    BufWriter::new(File::create(outfile).context("failed to open JSON outfile")?);
                serde_json::to_writer_pretty(&mut fp, &data)
                    .context("failed to dump JSON to file")?;
                fp.write_all(b"\n")
                    .context("failed to write final newline to JSON outfile")?;
                fp.flush().context("failed to flush JSON outfile")?;
            }
            Reporter::Csv => (),
        }
        Ok(())
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
struct StatReport {
    #[serde(with = "time::serde::rfc3339::option")]
    start_time: Option<OffsetDateTime>,
    #[serde(with = "time::serde::rfc3339::option")]
    end_time: Option<OffsetDateTime>,
    base_url: Url,
    traversals: Vec<Report>,
}

impl StatReport {
    fn new(base_url: Url) -> Self {
        StatReport {
            start_time: None,
            end_time: None,
            base_url,
            traversals: Vec::new(),
        }
    }
}
