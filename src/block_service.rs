use ethers::providers::{Middleware, Provider, Ws};
use ethers::types::{Block, Transaction, H160, H256, U64};
use tokio::sync::mpsc::{unbounded_channel, UnboundedReceiver};
use futures::{stream::SelectAll, StreamExt}; // still use futures for SelectAll
use std::collections::{HashSet, VecDeque};
use std::sync::Arc;

use crate::mult_provider::MultiProvider;

pub struct Snapshot {
    pub block_number: U64,
    pub pending_hashes: Vec<H256>,
    pub relevant_txs: Vec<Transaction>,
}

pub struct BlockService {
    
    ws_urls: Vec<String>,
    watch_addresses: HashSet<H160>,
}

impl BlockService {
    pub fn new(
        ws_urls: Vec<String>,
        watch_addresses: HashSet<H160>,
    ) -> Self {
        Self {
            ws_urls,
            watch_addresses,
        }
    }

 /// Returns a Receiver that gets one Snapshot per block.
    /// The actual subscriptions and HTTP calls run in a background task.
    pub fn spaw_new_block_subscription_service(self: Arc<Self>) -> UnboundedReceiver<Block<H256>> {
        // 1) Create the unbounded channel
        let (tx, rx) = unbounded_channel::<Block<H256>>();

        // 2) Spawn the background task
        tokio::spawn(async move {
            // 2a) Connect to each WS endpoint
            let mut ws_nodes = Vec::with_capacity(self.ws_urls.len());
            for url in &self.ws_urls {
                match Provider::<Ws>::connect(url).await {
                    Ok(ws) => ws_nodes.push(ws),
                    Err(e) => {
                        eprintln!("WS connect failed ({}): {}", url, e);
                    }
                }
            }

            // 2b) Merge all block subscriptions
            let mut merged = SelectAll::new();
            for ws in &ws_nodes {
                match ws.subscribe_blocks().await {
                    Ok(stream) => merged.push(stream),
                    Err(e) => {
                        eprintln!("subscribe_blocks failed: {}", e);
                    }
                }
            }

            // 2c) Drive the loop
            while let Some(header) = merged.next().await {
                // unwrap the block number              

                // send it (ignore if the receiver was dropped)
                let _ = tx.send(header);
            }
        });

        // 3) Return the receiver immediately
        rx
    }


}
