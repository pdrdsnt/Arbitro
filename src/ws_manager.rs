use std::{collections::HashSet, sync::Arc};

use ethers::types::{Log, H160};
use futures::{stream::{SelectAll, FuturesUnordered}, StreamExt};
use tokio::{
    sync::{
        mpsc::{unbounded_channel, UnboundedReceiver, UnboundedSender},
        Mutex, RwLock,
    },
    task::JoinHandle,
};
use tokio_stream::{wrappers::UnboundedReceiverStream, StreamMap};
use uuid::Uuid;

pub struct Subscription {
    pub receiver_id: Uuid,
    pub handle: JoinHandle<()>,
    pub tracking: HashSet<H160>,
}

pub struct SharedThing {
    subscriptions: Vec<Subscription>
}

pub struct WsManager {
    subscriptions: Arc<Mutex<Vec<Subscription>>>,
    new_sub_rx: Mutex<Option<UnboundedReceiver<UnboundedReceiver<Log>>>>,
    new_sub_tx: UnboundedSender<UnboundedReceiver<Log>>,
    pub funnel_rx: UnboundedReceiver<Log>,
    funnel_tx: UnboundedSender<Log>,
}

impl WsManager {
    pub fn new() -> Self {
        let (new_sub_tx, new_sub_rx) = unbounded_channel();
        let (funnel_tx, funnel_rx) = unbounded_channel();
        WsManager {
            subscriptions: Arc::new(Mutex::new(Vec::new())),
            new_sub_rx: Mutex::new(Some(new_sub_rx)),
            new_sub_tx,
            funnel_rx,
            funnel_tx,
        }
    }

    pub async fn start(&self) {
        let mut new_sub_rx = {
            let mut guard = self.new_sub_rx.lock().await;
            guard.take().expect("start called twice")
        };
        let mut funnel_tx = self.funnel_tx.clone();
        let mut select_all = SelectAll::default();

        tokio::spawn(async move {
            loop {
                tokio::select! {
                    Some(item) = merged.next() => {
                        let _ = funnel_tx.send(item);
                    }
                    Some(new_rx) = new_sub_rx.recv() => {
                        let stream = UnboundedReceiverStream::new(new_rx);
                        merged.push(stream);
                    }
                    else => break,
                }
            }
        });
    }

    pub async fn add_subscription(
        &self, receiver: UnboundedReceiver<Log>, handle: JoinHandle<()>, tracking: HashSet<H160>,
    ) {
        let mut subs = self.subscriptions.lock().await;
        subs.push(Subscription { receiver_id: uuid::Uuid::new_v4(), handle, tracking });
        drop(subs);
        let _ = self.new_sub_tx.send(receiver);
    }

    pub async fn remove_subscription(&self, pool: &H160) {
        let mut subs = self.subscriptions.lock().await;
        let mut i = 0;
        while i < subs.len() {
            if subs[i].tracking.contains(pool) {
                subs[i].handle.abort();
                subs.remove(i);
            } else {
                i += 1;
            }
        }
    }
}

pub struct Insider {
    subscriptions: Arc<Mutex<Vec<Subscription>>>,
}