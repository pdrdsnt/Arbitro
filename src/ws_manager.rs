use std::{collections::HashSet, sync::Arc};

use ethers::types::{Log, H160};
use futures::stream::StreamExt;
use tokio::{
    sync::{
        mpsc::{unbounded_channel, UnboundedReceiver, UnboundedSender},
        Mutex,
    },
    task::JoinHandle,
};
use tokio_stream::{wrappers::UnboundedReceiverStream, StreamMap};
use uuid::Uuid;

pub struct Subscription {
    pub receiver_id: Uuid,
}

pub enum Control<T: Send + 'static> {
    Add { id: Uuid, receiver: UnboundedReceiver<T>, handle: JoinHandle<()> },
    Rmv(Uuid),
}

pub struct WsManager<T: Send + 'static> {
    pub sub_tx: Option<UnboundedSender<Control<T>>>,
    pub funnel_rx: Option<UnboundedReceiver<T>>,
}

impl<T: Send + 'static> WsManager<T> {
    pub async fn start() -> Self {
        let subscriptions = Arc::new(Mutex::new(Vec::<Subscription>::new()));
        let (new_sub_tx, mut new_sub_rx) = unbounded_channel::<Control<T>>();
        let (funnel_tx, funnel_rx) = unbounded_channel::<T>();

        let subscriptions_clone = Arc::clone(&subscriptions);

        tokio::spawn(async move {
            let mut streams = StreamMap::<Uuid, UnboundedReceiverStream<T>>::new();
            let mut handles_map = std::collections::HashMap::<Uuid, JoinHandle<()>>::new();
            let mut active_ids = HashSet::<Uuid>::new();

            loop {
                tokio::select! {
                    Some(control) = new_sub_rx.recv() => {
                        match control {
                            Control::Add { id, receiver, handle } => {
                                streams.insert(id, UnboundedReceiverStream::new(receiver));
                                handles_map.insert(id, handle);
                                active_ids.insert(id);

                                let mut subs = subscriptions_clone.lock().await;
                                subs.push(Subscription {
                                    receiver_id: id,
                                });
                            }
                            Control::Rmv(id) => {
                                streams.remove(&id);
                                if let Some(h) = handles_map.remove(&id) {
                                    h.abort();
                                }
                                active_ids.remove(&id);

                                let mut subs = subscriptions_clone.lock().await;
                                subs.retain(|s| s.receiver_id != id);
                            }
                        }
                    }
                    Some((id, item)) = streams.next() => {
                        if active_ids.contains(&id) {
                            let _ = funnel_tx.send(item);
                        }
                    }
                    else => break,
                }
            }
        });

        WsManager { sub_tx: Some(new_sub_tx), funnel_rx: Some(funnel_rx) }
    }

    pub async fn add_subscription(
        &self, receiver: UnboundedReceiver<T>, handle: JoinHandle<()>,
    ) -> Option<Uuid> {
        if let Some(sub_tx) = self.sub_tx.as_ref() {
            let id = Uuid::new_v4();
            let control = Control::Add { id, receiver, handle };
            let _ = sub_tx.send(control);
            return Some(id);
        }
        None
    }

    pub async fn remove_subscription(&self, id: Uuid) {
        if let Some(sub_tx) = self.sub_tx.as_ref() {
            let control = Control::Rmv(id);
            let _ = sub_tx.send(control);
        }
    }
}
