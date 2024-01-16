mod test_service;

use std::sync::atomic::{AtomicUsize, Ordering};

use futures::{future::ready, Stream, StreamExt};
use test_service::*;

use crate::{configs::ResolvedConfig, response_info::ResponseInfo};
use anyhow::Result;
use tokio::time::Duration;
use tower::{
    buffer::Buffer,
    limit::{ConcurrencyLimit, RateLimit},
    ServiceExt,
};

use super::Cancellation;

#[derive(Clone)]
pub(crate) struct SpamService {
    service: Buffer<ConcurrencyLimit<RateLimit<TestService<reqwest::Client>>>, ResolvedConfig>,
    config: ResolvedConfig,
    cancellation: Cancellation,
}

impl SpamService {
    pub(crate) fn new(config: ResolvedConfig, cancellation: Cancellation) -> Result<Self> {
        let client = reqwest::Client::builder()
            .timeout(config.timeout)
            .pool_idle_timeout(Duration::from_secs(10))
            .build()?;

        let service = tower::ServiceBuilder::new()
            .buffer(100)
            .concurrency_limit(config.max_concurrent)
            .rate_limit(config.max_rps as u64, Duration::from_secs(1))
            .layer(TestLayer::new(client.clone()))
            .service(client);

        Ok(Self {
            service,
            config,
            cancellation,
        })
    }

    pub async fn run_test(&'_ mut self) -> impl Stream<Item = ResponseInfo> + '_ {
        let stream = ConfigStream {
            config: self.config.clone(),
            count: AtomicUsize::new(self.config.count),
            cancellation: self.cancellation.clone(),
        };
        let svc = self
            .service
            .ready()
            .await
            .expect("unable to wait for service ready");

        svc.call_all(stream).filter_map(|v| ready(v.ok()))
    }
}

struct ConfigStream {
    config: ResolvedConfig,
    cancellation: Cancellation,
    count: AtomicUsize,
}

impl Stream for ConfigStream {
    type Item = ResolvedConfig;

    fn poll_next(
        self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Option<Self::Item>> {
        let count = self.count.load(Ordering::Relaxed);
        if count > 0 && !self.cancellation.is_canceled() {
            match self.count.compare_exchange(
                count,
                count - 1,
                Ordering::Relaxed,
                Ordering::Relaxed,
            ) {
                Ok(_) => std::task::Poll::Ready(Some(self.config.clone())),
                Err(_) => {
                    cx.waker().wake_by_ref();
                    std::task::Poll::Pending
                }
            }
        } else {
            std::task::Poll::Ready(None)
        }
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        let count = self.count.load(Ordering::Relaxed);

        (count, Some(count))
    }
}
