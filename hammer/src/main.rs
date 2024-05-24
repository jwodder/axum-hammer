mod buffered_tasks;
use crate::buffered_tasks::BufferedTasks;
use clap::Parser;
use futures_util::TryStreamExt;
use std::num::NonZeroUsize;
use std::time::Instant;

#[derive(Clone, Debug, Eq, Parser, PartialEq)]
struct Arguments {
    url: url::Url,

    requests: NonZeroUsize,

    #[arg(required = true)]
    workers: Vec<NonZeroUsize>,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let args = Arguments::parse();
    for workqty in args.workers {
        let start = Instant::now();
        let tasks = BufferedTasks::from_iter(
            workqty.get(),
            std::iter::repeat(args.url.clone())
                .take(args.requests.get())
                .map(get_url),
        );
        tasks.try_collect::<()>().await?;
        let elapsed = start.elapsed();
        println!("{workqty} workers: {elapsed:?}");
    }
    Ok(())
}

async fn get_url(url: url::Url) -> anyhow::Result<()> {
    reqwest::get(url).await?.error_for_status()?;
    Ok(())
}
