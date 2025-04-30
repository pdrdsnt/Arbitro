
use ethers::types::{Transaction, Address, H256, Bytes};
use std::time::Duration;
use tokio::sync::mpsc;

/// A simplified view of a pending tx in the mempool
#[derive(Debug, Clone)]
pub struct MempoolTx {
    pub hash: H256,
    pub from: Address,
    pub to: Option<Address>,
    pub input: Bytes,
}

/// Configuration for which pools to watch
#[derive(Debug, Clone)]
pub struct PoolFilter {
    /// The set of pool contract addresses (V2/V3/V4) to track
    pub pool_addrs: Vec<Address>,
}

pub struct MempoolFetcher<P> {
    provider: ethers::providers::Provider<P>,
    filter: PoolFilter,
    poll_interval: Duration,
    seen: std::collections::HashSet<H256>,
    tx_sender: mpsc::Sender<MempoolTx>,
}

impl<P> MempoolFetcher<P>
where
    P: ethers::providers::JsonRpcClient + 'static + Send + Sync,
    ethers::providers::Provider<P>: Clone,
{
    /// Create a new fetcher.  
    /// `tx_sender` is the channel you’ll read from to get matched txs.
    pub fn new(
        provider: ethers::providers::Provider<P>,
        filter: PoolFilter,
        poll_interval: Duration,
        tx_sender: mpsc::Sender<MempoolTx>,
    ) -> Self {
        Self {
            provider,
            filter,
            poll_interval,
            seen: Default::default(),
            tx_sender,
        }
    }

    /// Calls the JSON-RPC `eth_pendingTransactions`
    async fn fetch_all(&self) -> Result<Vec<Transaction>, ethers::providers::ProviderError> {
        // no params → empty vec
        self.provider
            .request("eth_pendingTransactions", Vec::<String>::new())
            .await
    }

}
