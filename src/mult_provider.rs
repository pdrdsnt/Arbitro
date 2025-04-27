use axum::async_trait;
use ethers::providers::{JsonRpcClient, Provider, ProviderError};
use serde::de::DeserializeOwned;
use serde::Serialize;
use serde_json::Value;
use std::{
    sync::{Arc, RwLock},
    time::Duration,
    sync::atomic::{AtomicUsize, Ordering},
    fmt::Debug,
};

#[derive(Debug)]
pub struct MultiProvider {
    nodes: Arc<Vec<String>>, // or NodeState with backoff timestamps
    idx: AtomicUsize,
    backoff: Duration,
}

impl MultiProvider {
    pub fn new(urls: Vec<String>, backoff: Duration) -> Self {
        MultiProvider {
            nodes: Arc::new(urls),
            idx: AtomicUsize::new(0),
            backoff,
        }
    }

    /// Round-robin + backoff logic here
    async fn send_raw(&self, method: &str, params: Vec<Value>) 
        -> Result<Value, ProviderError>
    {
        let urls = self.nodes.clone();
        let n = urls.len();
        let start = self.idx.fetch_add(1, Ordering::Relaxed);
        for i in 0..n {
            let url = &urls[(start + i) % n];
            // build a temporary Http client for simplicity:
            let provider = Provider::try_from(url.as_str()).unwrap();
            match provider.request(method, params.clone()).await {
                Ok(res) => return Ok(res),
                Err(_) => tokio::time::sleep(self.backoff).await,
            }
        }
        Err(ProviderError::CustomError("all nodes failed".into()))
    }
}
#[async_trait]
impl JsonRpcClient for MultiProvider {
    type Error = ProviderError;

    async fn request<T, R>(
        &self,
        method: &str,
        params: T,
    ) -> Result<R, Self::Error>
    where
        // The *params* must be serializable so we can turn them into JSON…
        T: Debug + Serialize + Send + Sync,
        // …and the *response* must be deserializable (and Send so the future can move it).
        R: DeserializeOwned + Send,
    {
        // 1) turn your generic `params` into Vec<Value> (or whatever your send_raw() expects)
        let raw = serde_json::to_value(params)
            .map_err(ProviderError::SerdeJson)?;
        let params = raw
            .as_array()
            .cloned()
            .unwrap_or_default();

        // 2) call your rotation/back-off logic once, get back raw JSON
        let json = self.send_raw(method, params).await?;

        // 3) deserialize into the exact `R` the caller requested
        serde_json::from_value(json).map_err(ProviderError::SerdeJson)
    }
}