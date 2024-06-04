mod tasks;
use crate::tasks::request_tasks;
use anyhow::Context;
use clap::{Parser, Subcommand};
use futures_util::TryStreamExt;
use patharg::OutputArg;
use serde::Serialize;
use std::io::{BufWriter, Write};
use std::num::NonZeroUsize;
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
        #[arg(short = 'o', long, default_value_t)]
        outfile: OutputArg,

        url: Url,

        requests: NonZeroUsize,

        #[arg(required = true)]
        workers: Vec<NonZeroUsize>,
    },
    Subpages {
        #[arg(short = 'o', long, default_value_t)]
        outfile: OutputArg,

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
            outfile,
            url,
            requests,
            workers,
        } => (
            Reporter::new(outfile, url.clone()),
            Session::Repeat { url, requests },
            workers,
        ),
        Command::Subpages {
            outfile,
            url,
            workers,
            samples,
        } => (
            Reporter::new(outfile, url.clone()),
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

#[derive(Clone, Debug, Eq, PartialEq)]
struct Reporter {
    outfile: OutputArg,
    data: StatReport,
}

impl Reporter {
    fn new(outfile: OutputArg, base_url: Url) -> Self {
        Reporter {
            outfile,
            data: StatReport::new(base_url),
        }
    }

    fn start(&mut self) {
        self.data.start_time = Some(OffsetDateTime::now_utc());
    }

    fn process(&mut self, report: Report) {
        eprintln!(
            "Finished: workers = {}, requests = {}, elapsed = {:?}",
            report.workers,
            report.request_times.len(),
            report.overall_time
        );
        self.data.traversals.push(report);
    }

    fn end(mut self) -> anyhow::Result<()> {
        self.data.end_time = Some(OffsetDateTime::now_utc());
        let mut fp = BufWriter::new(
            self.outfile
                .create()
                .context("failed to open JSON outfile")?,
        );
        serde_json::to_writer_pretty(&mut fp, &self.data).context("failed to dump JSON to file")?;
        fp.write_all(b"\n")
            .context("failed to write final newline to JSON outfile")?;
        fp.flush().context("failed to flush JSON outfile")?;
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
