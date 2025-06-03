use std::{collections::HashSet, sync::Arc};

use ethers::{abi::Hash, prelude::*};
use futures::stream::{SelectAll, StreamExt};
use tokio::{
    sync::mpsc::{unbounded_channel, UnboundedReceiver, UnboundedSender},
    task::JoinHandle,
};
use uuid::Uuid;

use crate::ws_manager::{self, WsManager};

struct Chunk {
    addrs: HashSet<H160>,
    tombstones: HashSet<H160>,
    id: uuid::Uuid,
}

pub struct ChainDataService {
    pub ws_providers: Arc<Vec<Provider<Ws>>>,
    pub ws_manager: WsManager<Log>,
    pub monitoring: Vec<Chunk>,
}

impl ChainDataService {
    pub async fn new(
        ws_providers: Arc<Vec<Provider<Ws>>>, initial_addrs: impl IntoIterator<Item = H160>,
        collapse_threshold: usize,
    ) -> anyhow::Result<Self> {
        println!("start service");

        let ws_manager = WsManager { sub_tx: None, funnel_rx: None };
        let mut svc: ChainDataService = Self { ws_providers, ws_manager, monitoring: Vec::new() };

        Ok(svc)
    }

    /// Spawn block subscriber that forwards to `block_tx`
    pub fn spawn_block_subscriber(&self) -> UnboundedReceiver<Block<H256>> {
        let (block_tx, block_rx) = unbounded_channel();
        let providers = self.ws_providers.clone();

        tokio::spawn(async move {
            let mut merged = SelectAll::new();
            for ws in &*providers {
                if let Ok(stream) = ws.subscribe_blocks().await {
                    merged.push(stream);
                }
            }
            while let Some(header) = merged.next().await {
                let _ = block_tx.send(header);
            }
        });

        block_rx
    }

    fn spawn_log_subsubscriber(&self, chunk: &Chunk) -> (JoinHandle<()>, UnboundedReceiver<Log>) {
        let filter = Filter::new()
            .from_block(BlockNumber::Latest)
            .address(ValueOrArray::Array(chunk.addrs.clone().into_iter().collect()));

        let (log_tx, log_rx) = unbounded_channel();
        let providers = self.ws_providers.clone();
        let m = self.ws_manager.funnel_rx.as_ref().unwrap();
        let j = tokio::spawn(async move {
            let mut merged = SelectAll::new();
            for ws in &*providers {
                if let Ok(stream) = ws.subscribe_logs(&filter).await {
                    merged.push(stream);
                }
            }
            while let Some(header) = merged.next().await {
                let _ = log_tx.send(header);
            }
        });

        (j, log_rx)
    }

    pub fn spawn_log_subscriber(&self, filter: Filter) -> UnboundedReceiver<Log> {
        let (log_tx, log_rx) = unbounded_channel();
        let providers = self.ws_providers.clone();
        let m = self.ws_manager.funnel_rx.as_ref().unwrap();
        tokio::spawn(async move {
            let mut merged = SelectAll::new();
            for ws in &*providers {
                if let Ok(stream) = ws.subscribe_logs(&filter).await {
                    merged.push(stream);
                }
            }
            while let Some(header) = merged.next().await {
                let _ = log_tx.send(header);
            }
        });

        log_rx
    }

    pub fn add_pool(&mut self, pool: H160) {
        self.monitoring.push(Chunk {
            addrs: HashSet::<H160>::from_iter([pool.clone()]),
            tombstones: HashSet::new(),
            id: uuid::Uuid::new_v4(),
        });

        self.ws_manager.add_subscription(todo!(), todo!());
    }

    pub fn check(&mut self) {
        let len = self.monitoring.len();
        if len > 6 {
            let mut addr = HashSet::<H160>::new();
            let mut rmv = HashSet::<H160>::new();

            for chunk in self.monitoring.iter() {
                addr.extend(&chunk.addrs);
                rmv.extend(&chunk.tombstones);
                self.ws_manager.remove_subscription(chunk.id);
            }

            // Get elements in addr but not in rmv
            let final_addr: HashSet<H160> = addr.difference(&rmv).cloned().collect();

            let new_chunk =
                Chunk { addrs: final_addr, tombstones: HashSet::new(), id: Uuid::new_v4() };

            self.spawn_log_subsubscriber(&new_chunk);
        }
    }

    /// Spawn mempool subscriber that forwards tx hashes to `mempool_tx`
    pub fn spawn_mempool_subscriber(&self) -> UnboundedReceiver<Transaction> {
        let providers = self.ws_providers.clone();
        let (mempool_tx, mempool_rx) = unbounded_channel();

        tokio::spawn(async move {
            let mut merged = SelectAll::new();
            for ws in &*providers {
                if let Ok(stream) = ws.subscribe_full_pending_txs().await {
                    merged.push(stream);
                }
            }
            while let Some(tx_hash) = merged.next().await {
                let _ = mempool_tx.send(tx_hash);
            }
        });

        mempool_rx
    }
}
