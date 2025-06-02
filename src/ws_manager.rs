use std::{collections::HashSet, sync::Arc};
use ethers::types::{Log, H160};
use futures::StreamExt;
use tokio::{
    sync::{
        mpsc::{unbounded_channel, UnboundedReceiver, UnboundedSender},
        Mutex,
    },
    task::JoinHandle,
};
use tokio_stream::{wrappers::UnboundedReceiverStream, StreamMap};

pub struct Subscription {
    pub receiver: UnboundedReceiver<Log>,
    pub handle: JoinHandle<()>,
    pub tracking: HashSet<H160>,
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
        let mut merged: StreamMap<uuid::Uuid, UnboundedReceiverStream<Log>> = StreamMap::new();

        tokio::spawn(async move {
            loop {
                tokio::select! {
                    Some((id, log)) = async {
                        let mut next_fut = None;
                        for (key, stream) in merged.iter_mut() {
                            let mut s = stream.by_ref();
                            if let Some(item) = s.next().await {
                                next_fut = Some((key.clone(), item));
                                break;
                            }
                        }
                        next_fut
                    } => {
                        let _ = funnel_tx.send(log);
                    }
                    Some(new_rx) = new_sub_rx.recv() => {
                        let id = uuid::Uuid::new_v4();
                        let stream = UnboundedReceiverStream::new(new_rx);
                        merged.insert(id, stream);
                    }
                    else => break,
                }
            }
        });
    }

    pub async fn add_subscription(
        &self,
        receiver: UnboundedReceiver<Log>,
        handle: JoinHandle<()>,
        tracking: HashSet<H160>,
    ) {
        let mut subs = self.subscriptions.lock().await;
        subs.push(Subscription {
            receiver: receiver.clone(),
            handle,
            tracking,
        });
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
