use reqwest::Url;
use futures::Future;
use backoff::{future::retry_notify, Error::{Transient, Permanent}};
use std::time::Duration;
use anyhow::format_err;
use super::{LogContent, RouterResult};


#[derive(Debug, Clone)]
pub struct RouterApiClient {
    old_router_url: Url,
    new_router_url: Url,
    http_client: reqwest::Client,
}

impl RouterApiClient {
    pub fn new(old_router_url: Url, new_router_url: Url, req_server_timeout: Duration) -> Self {
        let http_client = reqwest::ClientBuilder::new()
            .timeout(req_server_timeout)
            .build()
            .expect("Failed to create request client");
        log::info!("RouterApiClient server urls:{}, {}", old_router_url, new_router_url);
        Self {
            old_router_url,
            new_router_url,
            http_client,
        }
    }


    async fn with_retries<I, E, Fn, Fut>(&self, operation: Fn) -> anyhow::Result<I>
        where
            Fn: FnMut() -> Fut,
            Fut: Future<Output=Result<I, backoff::Error<E>>>,
            E: std::fmt::Display, {
        let notify = |err, next_after: Duration| {
            let duration_secs = next_after.as_millis() as f32 / 1000.0f32;
            log::warn!(
               "Failed to reach server err: <{}>, retrying after: {:.1}s",
                err,
                duration_secs,
            )
        };

        retry_notify(Self::get_backoff(), operation, notify)
            .await
            .map_err(|e| {
                format_err!(
                    "Prover can't reach server, for the max elapsed time of the backoff: {}",
                    e
                )
            })
    }

    fn get_backoff() -> backoff::ExponentialBackoff {
        backoff::ExponentialBackoff {
            current_interval: Duration::from_secs(1),
            initial_interval: Duration::from_secs(1),
            multiplier: 1.5f64,
            max_interval: Duration::from_secs(2 * 60),
            max_elapsed_time: Some(Duration::from_secs(2 * 60)),
            ..Default::default()
        }
    }

    pub async fn call_old_router(&mut self, log_content: &LogContent) -> anyhow::Result<RouterResult> {
        gen_url(&mut self.old_router_url, log_content);
        self.call_router(&self.old_router_url).await
    }

    pub async fn call_new_router(&mut self, log_content: &LogContent) -> anyhow::Result<RouterResult> {
        gen_url(&mut self.new_router_url, log_content);
        self.call_router(&self.new_router_url).await
    }

    async fn call_router(&self, url: &Url) -> anyhow::Result<RouterResult> {
        let operation = || async {
            let response = self
                .http_client
                .get(url.clone())
                .send()
                .await
                .map_err(|e| format_err!("failed to send call router request: {}", e))?;
            if response.status() != reqwest::StatusCode::OK {
                return Err(Transient(format_err!("router request error:{:?}", response.status())));
            }
            response
                .json()
                .await
                .map_err(|e| Permanent(format_err!("failed parse json on RouterResult request: {}", e)))
        };
        self.with_retries(operation).await
    }
}

fn gen_url(url: &mut Url, log_content: &LogContent) {
    let res = format!(
        "fromToken={}&fromTokenAddr={}&toToken={}&toTokenAddr={}&inAmount={}&fromDecimal={}&toDecimal={}",
        log_content.from_token,
        log_content.from_token_addr,
        log_content.to_token,
        log_content.to_token_addr,
        log_content.in_amount,
        log_content.from_decimal,
        log_content.to_decimal
    );
    url.set_query(Some(res.as_str()))
}