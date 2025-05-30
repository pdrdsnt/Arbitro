use std::{collections::HashSet, sync::Arc};

use ethers::prelude::*;
use futures::stream::{SelectAll, StreamExt};
use tokio::{
    sync::mpsc::{unbounded_channel, UnboundedReceiver, UnboundedSender},
    task::JoinHandle,
};

struct Chunk {
    addrs: HashSet<H160,>,
    handle: JoinHandle<(),>,
    tombstones: HashSet<H160,>,
}

pub struct ChainDataService {
    pub ws_providers: Arc<Vec<Provider<Ws,>,>,>,
    // chunks: Vec<Chunk>,
    // all_addrs: HashSet<H160>,
    // collapse_threshold: usize,
}

impl ChainDataService {
    pub async fn new(
        ws_urls: Vec<String,>, initial_addrs: impl IntoIterator<Item = H160,>, collapse_threshold: usize,
    ) -> anyhow::Result<Self,> {
        let mut providers = Vec::with_capacity(ws_urls.len(),);
        for url in ws_urls {
            match Provider::<Ws,>::connect(&url,).await {
                Ok(ws,) => providers.push(ws,),
                Err(err,) => eprintln!("Failed WS connect to {}: {}", url, err),
            }
        }
        if providers.is_empty() {
            anyhow::bail!("Could not connect to any WS endpoint");
        }

        let ws_providers = Arc::new(providers,);

        let mut svc = Self { ws_providers, };

        Ok(svc,)
    }
    /// Spawn block subscriber that forwards to `block_tx`
    pub fn spawn_block_subscriber(&self,) -> UnboundedReceiver<Block<H256,>,> {
        let (block_tx, block_rx,) = unbounded_channel();
        let providers = self.ws_providers.clone();

        tokio::spawn(async move {
            let mut merged = SelectAll::new();
            for ws in &*providers {
                if let Ok(stream,) = ws.subscribe_blocks().await {
                    merged.push(stream,);
                }
            }
            while let Some(header,) = merged.next().await {
                let _ = block_tx.send(header,);
            }
        },);

        block_rx
    }

    pub fn spawn_log_subscriber(&self, filter: Filter,) -> UnboundedReceiver<Log,> {
        let (log_tx, log_rx,) = unbounded_channel();
        let providers = self.ws_providers.clone();

        tokio::spawn(async move {
            let mut merged = SelectAll::new();
            for ws in &*providers {
                if let Ok(stream,) = ws.subscribe_logs(&filter,).await {
                    merged.push(stream,);
                }
            }
            while let Some(header,) = merged.next().await {
                let _ = log_tx.send(header,);
            }
        },);

        log_rx
    }
    /// Spawn log subscription chunk
    // i dont need that for now and idk how to implement in a idiomatic way
    // fn spawn_chunk(&mut self, _addrs: Option<&HashSet<H160>>) {
    // let addrs = match _addrs {
    // Some(addrs) => addrs,
    // None => &self.all_addrs,
    // };
    // let filter = Filter::new().address(addrs.iter().cloned().collect::<Vec<_>>());
    //
    // let providers = self.ws_providers.clone();
    // let log_tx = self.log_tx.clone();
    //
    // let handle = tokio::spawn(async move {
    // let mut merged = SelectAll::new();
    // for ws in &*providers {
    // if let Ok(stream) = ws.subscribe_logs(&filter).await {
    // merged.push(stream);
    // }
    // }
    // while let Some(log) = merged.next().await {
    // let _ = log_tx.send(log);
    // }
    // });
    //
    // self.chunks.push(Chunk {
    // addrs: addrs.clone(),
    // handle,
    // tombstones: HashSet::new(),
    // });
    // }
    //
    // pub fn add_pool(&mut self, addr: H160) {
    // self.all_addrs.insert(addr);
    // let mut singleton = HashSet::new();
    // singleton.insert(addr);
    // self.spawn_chunk(Some(&singleton));
    //
    // if self.chunks.len() >= self.collapse_threshold {
    // self.collapse_chunks();
    // }
    // }
    //
    // pub fn remove_pool(&mut self, addr: &H160) {
    // self.all_addrs.remove(addr);
    // for chunk in &mut self.chunks {
    // if chunk.addrs.contains(addr) {
    // chunk.tombstones.insert(*addr);
    // }
    // }
    // }
    //
    // fn collapse_chunks(&mut self) {
    // for chunk in self.chunks.drain(..) {
    // chunk.handle.abort();
    // }
    // self.spawn_chunk(None);
    // }

    /// Spawn mempool subscriber that forwards tx hashes to `mempool_tx`
    pub fn spawn_mempool_subscriber(&self,) -> UnboundedReceiver<Transaction,> {
        let providers = self.ws_providers.clone();
        let (mempool_tx, mempool_rx,) = unbounded_channel();

        tokio::spawn(async move {
            let mut merged = SelectAll::new();
            for ws in &*providers {
                if let Ok(stream,) = ws.subscribe_full_pending_txs().await {
                    merged.push(stream,);
                }
            }
            while let Some(tx_hash,) = merged.next().await {
                let _ = mempool_tx.send(tx_hash,);
            }
        },);

        mempool_rx
    }
}
