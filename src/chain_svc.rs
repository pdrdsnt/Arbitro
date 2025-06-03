use std::{collections::HashSet, sync::Arc};

use ethers::{abi::Hash, prelude::*};
use futures::{
    future::join_all,
    stream::{SelectAll, StreamExt},
};
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
        ws_providers: Arc<Vec<Provider<Ws>>>,
        initial_addrs: impl IntoIterator<Item = H160>,
        collapse_threshold: usize,
    ) -> anyhow::Result<Self> {
        println!("[ChainDataService] Starting service with collapse_threshold: {}", collapse_threshold);
        let addrs: HashSet<H160> = initial_addrs.into_iter().collect();
        println!("[ChainDataService] Initial addresses to monitor: {:?}", addrs);

        let ws_manager = WsManager { sub_tx: None };
        let svc: ChainDataService = Self { ws_providers, ws_manager, monitoring: Vec::new() };

        println!("[ChainDataService] Service initialized");
        Ok(svc)
    }

    /// Spawn block subscriber that forwards to `block_tx`
    pub fn spawn_block_subscriber(&self) -> UnboundedReceiver<Block<H256>> {
        let (block_tx, block_rx) = unbounded_channel();
        let providers = self.ws_providers.clone();

        tokio::spawn(async move {
            println!("[BlockSubscriber] Spawning block subscriber");
            let mut merged = SelectAll::new();
            for (idx, ws) in providers.iter().enumerate() {
                match ws.subscribe_blocks().await {
                    Ok(stream) => {
                        println!("[BlockSubscriber] Provider {} subscribed to blocks", idx);
                        merged.push(stream);
                    }
                    Err(e) => println!("[BlockSubscriber] Failed to subscribe provider {}: {:?}", idx, e),
                }
            }
            while let Some(header) = merged.next().await {
                println!("[BlockSubscriber] Received block: {:?}", header.number);
                let _ = block_tx.send(header);
            }
            println!("[BlockSubscriber] Block subscription stream ended");
        });

        block_rx
    }

    fn spawn_log_subsubscriber(&self, chunk: &Chunk) -> (JoinHandle<()>, UnboundedReceiver<Log>) {
        println!("[LogSubSubscriber] Spawning log subscriber for chunk ID: {}", chunk.id);
        println!("[LogSubSubscriber] Monitoring addresses: {:?}", chunk.addrs);
        let filter = Filter::new()
            .from_block(BlockNumber::Latest)
            .address(ValueOrArray::Array(chunk.addrs.clone().into_iter().collect()));

        let (log_tx, log_rx) = unbounded_channel();
        let providers = self.ws_providers.clone();
        let chunk_id = chunk.id;
        let j = tokio::spawn(async move {
            println!("[LogSubSubscriber:{}] Started background task", chunk_id);
            let mut merged = SelectAll::new();
            for (idx, ws) in providers.iter().enumerate() {
                match ws.subscribe_logs(&filter).await {
                    Ok(stream) => {
                        println!("[LogSubSubscriber:{}] Provider {} subscribed to logs", chunk_id, idx);
                        merged.push(stream);
                    }
                    Err(e) => println!("[LogSubSubscriber:{}] Failed to subscribe provider {}: {:?}", chunk_id, idx, e),
                }
            }
            while let Some(log) = merged.next().await {
                println!("[LogSubSubscriber:{}] Received log: Address={:?}, TxHash={:?}", chunk_id, log.address, log.transaction_hash);
                let _ = log_tx.send(log);
            }
            println!("[LogSubSubscriber:{}] Log subscription stream ended", chunk_id);
        });

        (j, log_rx)
    }

    pub async fn spawn_log_subscriber(&mut self) -> UnboundedReceiver<Log> {
        println!("[ChainDataService] Starting WsManager for log subscriptions");
        let (man, rx) = WsManager::<Log>::start().await;
        self.ws_manager = man;
        println!("[ChainDataService] WsManager started");
        rx
    }

    pub fn add_pool(&mut self, pool: H160) {
        println!("[ChainDataService] Adding pool to monitoring: {:?}", pool);
        let new_chunk = Chunk {
            addrs: HashSet::<H160>::from_iter([pool.clone()]),
            tombstones: HashSet::new(),
            id: uuid::Uuid::new_v4(),
        };

        println!("[ChainDataService] New chunk ID: {}", new_chunk.id);
        let (h, r) = self.spawn_log_subsubscriber(&new_chunk);
        self.ws_manager.add_subscription(r, h);
        println!("[ChainDataService] Subscription added to WsManager for chunk {}", new_chunk.id);

        self.monitoring.push(new_chunk);
        println!("[ChainDataService] Total chunks being monitored: {}", self.monitoring.len());
        // After adding pool, check if collapsing is needed
        let _ = futures::executor::block_on(self.check());
    }

    pub async fn check(&mut self) {
        let len = self.monitoring.len();
        println!("[ChainDataService] Checking monitoring chunks, count: {}", len);
        if len > 6 {
            println!("[ChainDataService] Exceeded collapse threshold, collapsing chunks");
            let mut addr = HashSet::<H160>::new();
            let mut rmv = HashSet::<H160>::new();
            let mut rmv_all = Vec::with_capacity(len);
            for chunk in self.monitoring.iter() {
                println!("[ChainDataService] Processing chunk ID: {}", chunk.id);
                addr.extend(&chunk.addrs);
                rmv.extend(&chunk.tombstones);
                println!("[ChainDataService] Removing subscription for chunk ID: {}", chunk.id);
                rmv_all.push(self.ws_manager.remove_subscription(chunk.id));
            }

            let final_addr: HashSet<H160> = addr.difference(&rmv).cloned().collect();
            println!("[ChainDataService] Final addresses after collapse: {:?}", final_addr);

            let new_chunk = Chunk { addrs: final_addr.clone(), tombstones: HashSet::new(), id: Uuid::new_v4() };
            println!("[ChainDataService] New collapsed chunk ID: {}", new_chunk.id);

            let (h, r) = self.spawn_log_subsubscriber(&new_chunk);
            self.ws_manager.add_subscription(r, h);
            println!("[ChainDataService] Added collapsed chunk subscription");

            join_all(rmv_all).await;
            self.monitoring.clear();
            self.monitoring.push(new_chunk);
            println!("[ChainDataService] Monitoring reset to single collapsed chunk");
        }
    }

    /// Spawn mempool subscriber that forwards tx hashes to `mempool_tx`
    pub fn spawn_mempool_subscriber(&self) -> UnboundedReceiver<Transaction> {
        let providers = self.ws_providers.clone();
        let (mempool_tx, mempool_rx) = unbounded_channel();

        tokio::spawn(async move {
            println!("[MempoolSubscriber] Spawning mempool subscriber");
            let mut merged = SelectAll::new();
            for (idx, ws) in providers.iter().enumerate() {
                match ws.subscribe_full_pending_txs().await {
                    Ok(stream) => {
                        println!("[MempoolSubscriber] Provider {} subscribed to mempool txs", idx);
                        merged.push(stream);
                    }
                    Err(e) => println!("[MempoolSubscriber] Failed to subscribe provider {}: {:?}", idx, e),
                }
            }
            while let Some(tx) = merged.next().await {
                println!("[MempoolSubscriber] Received pending tx: {:?}", tx.hash);
                let _ = mempool_tx.send(tx);
            }
            println!("[MempoolSubscriber] Mempool subscription stream ended");
        });

        mempool_rx
    }
}
