use std::{
    collections::HashMap,
    task::{Context, Poll},
};

use crate::{configs::ResolvedConfig, response_info::ResponseInfo};
use anyhow::Result;
use futures::future::BoxFuture;
use tokio::time::{Duration, Instant};
use tower::{BoxError, Layer, Service};

use crate::spam::create_request::build_reqwest;

#[derive(Clone)]
pub struct TestService<S> {
    inner: S,
    reqwest_client: reqwest::Client,
}

impl<S> Service<ResolvedConfig> for TestService<S>
where
    S: Service<reqwest::Request, Response = reqwest::Response, Error = reqwest::Error>
        + Clone
        + Send
        + 'static,
    S::Error: Into<BoxError> + ToString + Send + Sync + 'static,
    S::Future: Send + 'static,
{
    type Response = ResponseInfo;
    type Error = S::Error;
    type Future = BoxFuture<'static, Result<Self::Response, Self::Error>>;

    fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        self.inner.poll_ready(cx)
    }

    fn call(
        &mut self,
        ResolvedConfig {
            check_for,
            collect,
            latency_header,
            request,
            rotate_uuids,
            ..
        }: ResolvedConfig,
    ) -> Self::Future {
        let req = build_reqwest(&self.reqwest_client, request, rotate_uuids)
            .build()
            .unwrap(); // FIXME

        let start = Instant::now();
        let mut inner = self.inner.clone();
        Box::pin(async move {
            let res = inner.call(req).await;
            let time = start.elapsed();

            let res = match res {
                Ok(res) => res,
                Err(e) => {
                    return Ok(ResponseInfo::error(time, e.to_string(), None, None));
                }
            };

            let collected: HashMap<String, String> = collect
                .as_ref()
                .unwrap_or(&vec![])
                .iter()
                .map(|v| (v.to_owned(), get_header(&res, v)))
                .collect();

            let server_latency = latency_header.as_ref().map(|v| {
                let latency = get_header(&res, v);
                let latency = latency.parse::<u64>().unwrap_or_default();
                Duration::from_millis(latency)
            });

            let Some(items) = check_for else {
                return Ok(ResponseInfo::success(time, server_latency, collected));
            };

            let Ok(text) = res.text().await else {
                return Ok(ResponseInfo::error(
                    time,
                    "text content unavailable from response".into(),
                    server_latency,
                    Some(collected),
                ));
            };

            let unmatched: Vec<_> = items.into_iter().filter(|v| !text.contains(v)).collect();
            if unmatched.is_empty() {
                Ok(ResponseInfo::success(time, server_latency, collected))
            } else {
                Ok(ResponseInfo::error(
                    time,
                    format!("Missing values {unmatched:?}"),
                    server_latency,
                    Some(collected),
                ))
            }
        })
    }
}

fn get_header(res: &reqwest::Response, key: &str) -> String {
    res.headers()
        .get(key)
        .map(|h| h.to_str().unwrap_or("").to_owned())
        .unwrap_or_default()
}

pub struct TestLayer(reqwest::Client);

impl TestLayer {
    pub fn new(reqwest_client: reqwest::Client) -> Self {
        TestLayer(reqwest_client)
    }
}

impl<S> Layer<S> for TestLayer {
    type Service = TestService<S>;

    fn layer(&self, service: S) -> TestService<S> {
        TestService {
            inner: service,
            reqwest_client: self.0.clone(),
        }
    }
}
