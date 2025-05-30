use std::{
    cmp::{Ordering, Reverse},
    fmt::Debug,
    pin::Pin,
    sync::Arc,
    time::{Duration, Instant},
};

use async_trait::async_trait;
use ethers::types::{Block, Transaction, H256, U64};
use ethers_providers::{JsonRpcClient, Middleware, Provider, ProviderError, PubsubClient, SubscriptionStream, Ws};
use futures::{
    future::{BoxFuture, FutureExt, SelectAll},
    stream::select_all,
    Stream,
};
use rayon::vec;
use serde::{de::DeserializeOwned, Serialize};
use serde_json::{value::RawValue, Value};
use thiserror::Error;
use tokio::sync::Mutex;

#[derive(Debug,)]
struct NodeState {
    url: String,
    last_failure: Mutex<Option<Instant,>,>,
}

/// Errors that can occur in MultiProvider
#[derive(Debug, Error,)]
pub enum MultiProviderError {
    #[error("No providers available")]
    NoProviders,
    #[error("All providers are in backoff period")]
    AllBackoff,
    #[error("Request failed on all attempts: {0}")]
    RequestFailed(String,),
    #[error("Subscription failed: {0}")]
    SubscriptionFailed(String,),
}

impl ethers_providers::RpcError for MultiProviderError {
    fn is_error_response(&self,) -> bool {
        match self {
            MultiProviderError::RequestFailed(_,) => true,
            _ => false,
        }
    }

    fn is_serde_error(&self,) -> bool {
        match self {
            MultiProviderError::RequestFailed(_,) => false,
            _ => true,
        }
    }

    fn as_error_response(&self,) -> Option<&ethers_providers::JsonRpcError,> { todo!() }

    fn as_serde_error(&self,) -> Option<&serde_json::Error,> { todo!() }
}

impl From<MultiProviderError,> for ethers_providers::ProviderError {
    fn from(err: MultiProviderError,) -> Self { ethers_providers::ProviderError::CustomError(err.to_string(),) }
}

#[derive(Debug, Clone,)]
pub struct MultiProvider {
    nodes: Arc<Vec<NodeState,>,>,
    backoff: Duration,
    max_total_tries: usize,
}

impl MultiProvider {
    pub fn new(urls: Vec<String,>, backoff: Duration, max_total_tries: usize,) -> Self {
        let nodes = urls
            .into_iter()
            .map(|url| NodeState {
                url,
                last_failure: Mutex::new(None,),
            },)
            .collect();

        MultiProvider {
            nodes: Arc::new(nodes,),
            backoff,
            max_total_tries,
        }
    }
    /// helper to call eth_blockNumber over HTTP
    async fn get_block_number(&self,) -> Result<U64, MultiProviderError,> {
        // just wrap the JsonRpcClient impl:
        self.request("eth_blockNumber", Vec::<Value,>::new(),).await
    }

    /// Sends an RPC request, retrying across nodes with backoff
    pub async fn send_raw(&self, method: &str, params: Vec<Value,>,) -> Result<Value, MultiProviderError,> {
        if self.nodes.is_empty() {
            return Err(MultiProviderError::NoProviders,);
        }

        let mut last_error: Option<String,> = None;
        let mut tries = 0;

        // Keep trying until max_total_tries
        while tries < self.max_total_tries {
            tries += 1;
            // Build a list of candidate indices, filtering out backoff
            let now = Instant::now();
            let mut candidates = Vec::new();
            for (i, node,) in self.nodes.iter().enumerate() {
                let failure_time = *node.last_failure.lock().await;
                if failure_time.map_or(true, |t| now.duration_since(t,) >= self.backoff,) {
                    candidates.push(i,);
                }
            }

            if candidates.is_empty() {
                // No nodes available yet, wait for the soonest backoff to expire
                let earliest = {
                    let mut futures = Vec::new();
                    for n in self.nodes.iter() {
                        futures.push(async { n.last_failure.lock().await.map(|t| t + self.backoff,) },);
                    }
                    let results = futures::future::join_all(futures,).await;
                    results.into_iter().flatten().min()
                };

                if let Some(when,) = earliest {
                    let wait_dur = when.saturating_duration_since(Instant::now(),);
                    tokio::time::sleep(wait_dur,).await;
                    continue;
                } else {
                    return Err(MultiProviderError::AllBackoff,);
                }
            }

            // Rotate through candidates
            for &idx in &candidates {
                let node = &self.nodes[idx];
                let client = Provider::try_from(node.url.clone(),)
                    .map_err(|e| MultiProviderError::RequestFailed(e.to_string(),),)?;

                match client.request::<_, Value>(method, params.clone(),).await {
                    Ok(resp,) => return Ok(resp,),
                    Err(e,) => {
                        // record failure time
                        let mut lock = node.last_failure.lock().await;
                        *lock = Some(Instant::now(),);
                        last_error = Some(e.to_string(),);
                        // try next node
                    },
                }

                // check if we've exhausted tries
                if tries >= self.max_total_tries {
                    break;
                }
            }
        }

        Err(MultiProviderError::RequestFailed(last_error.unwrap_or_else(|| "Unknown".into(),),),)
    }
}

#[async_trait]
impl JsonRpcClient for MultiProvider {
    type Error = MultiProviderError;

    async fn request<T, R,>(&self, method: &str, params: T,) -> Result<R, Self::Error,>
    where
        T: Debug + Serialize + Send + Sync,
        R: DeserializeOwned + Send, {
        let raw = serde_json::to_value(params,).map_err(|e| MultiProviderError::RequestFailed(e.to_string(),),)?;
        let params = raw.as_array().cloned().unwrap_or_default();

        let json = self.send_raw(method, params,).await?;
        serde_json::from_value(json,).map_err(|e| MultiProviderError::RequestFailed(e.to_string(),),)
    }
}
