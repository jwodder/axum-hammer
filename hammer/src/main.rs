mod buffered_tasks;
use crate::buffered_tasks::BufferedTasks;
use clap::{Parser, Subcommand};
use futures_util::TryStreamExt;
use std::num::NonZeroUsize;
use std::time::Instant;
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
    match Arguments::parse().command {
        Command::Run {
            url,
            requests,
            workers,
        } => {
            for workqty in workers {
                let start = Instant::now();
                let tasks = BufferedTasks::from_iter(
                    workqty.get(),
                    std::iter::repeat(url.clone())
                        .take(requests.get())
                        .map(get_url),
                );
                tasks.try_collect::<()>().await?;
                let elapsed = start.elapsed();
                println!("{workqty} workers: {elapsed:?}");
            }
        }
        Command::Subpages { url, workers } => {
            for workqty in workers {
                let start = Instant::now();
                let r = reqwest::get(url.clone()).await?.error_for_status()?;
                let mut qty = 1;
                let body = r.text().await?;
                let mut tasks = BufferedTasks::from_iter(
                    workqty.get(),
                    body.lines().map(|key| get_url(urljoin(&url, [key]))),
                );
                while tasks.try_next().await? == Some(()) {
                    qty += 1;
                }
                let elapsed = start.elapsed();
                println!("{workqty} workers, {qty} requests: {elapsed:?}");
            }
        }
    }
    Ok(())
}

async fn get_url(url: Url) -> anyhow::Result<()> {
    reqwest::get(url).await?.error_for_status()?;
    Ok(())
}

fn urljoin<I>(url: &Url, segments: I) -> Url
where
    I: IntoIterator,
    I::Item: AsRef<str>,
{
    let mut url = url.clone();
    url.path_segments_mut()
        .expect("URL should be able to be a base")
        .pop_if_empty()
        .extend(segments);
    url
}
