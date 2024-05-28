use anyhow::Context as _;
use futures_util::Stream;
use std::collections::VecDeque;
use std::pin::Pin;
use std::sync::{Arc, Mutex};
use std::task::{Context, Poll};
use std::time::{Duration, Instant};
use tokio::{
    sync::mpsc::{channel, Receiver},
    task::AbortHandle,
};
use url::Url;

#[derive(Debug)]
#[must_use = "streams do nothing unless polled"]
pub(crate) struct RequestTasks {
    killers: Vec<AbortHandle>,
    receiver: Receiver<anyhow::Result<Duration>>,
}

pub(crate) fn request_tasks<I: IntoIterator<Item = Url>>(
    limit: usize,
    urls: I,
) -> anyhow::Result<RequestTasks> {
    let mut clients = Vec::with_capacity(limit);
    for _ in 0..limit {
        clients.push(
            reqwest::Client::builder()
                .build()
                .context("failed to create client")?,
        );
    }
    let (sender, receiver) = channel(32);
    let jobs = urls.into_iter().collect::<JobQueue>();
    let mut killers = Vec::with_capacity(limit);
    for client in clients {
        let sender = sender.clone();
        let jobs = jobs.clone();
        let handle = tokio::spawn(async move {
            while let Some(url) = jobs.pop_next() {
                if sender.send(get_url(&client, url).await).await.is_err() {
                    break;
                }
            }
        });
        killers.push(handle.abort_handle());
    }
    Ok(RequestTasks { killers, receiver })
}

impl Drop for RequestTasks {
    fn drop(&mut self) {
        for k in &self.killers {
            k.abort();
        }
    }
}

impl Stream for RequestTasks {
    type Item = anyhow::Result<Duration>;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        self.receiver.poll_recv(cx)
    }
}

#[derive(Clone, Debug)]
// The Mutex isn't held across `await`s, so use the std Mutex:
struct JobQueue(Arc<Mutex<VecDeque<Url>>>);

impl JobQueue {
    fn pop_next(&self) -> Option<Url> {
        let mut data = self.0.lock().expect("Mutex should not have been poisoned");
        data.pop_front()
    }
}

impl FromIterator<Url> for JobQueue {
    fn from_iter<I: IntoIterator<Item = Url>>(iter: I) -> JobQueue {
        JobQueue(Arc::new(Mutex::new(iter.into_iter().collect())))
    }
}

async fn get_url(client: &reqwest::Client, url: Url) -> anyhow::Result<Duration> {
    let start = Instant::now();
    let r = client.get(url).send().await?.error_for_status()?;
    let _ = r.bytes().await?;
    Ok(start.elapsed())
}
