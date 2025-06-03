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
}

pub enum Control<T: Send + 'static> {
    Add(UnboundedReceiver<T>),
    Rmv(Uuid),
}
pub struct WsManager<T: Send + 'static> {
    pub sub_tx: UnboundedSender<Control<T>>,
    pub funnel_rx: UnboundedReceiver<T>
}

impl<T: Send + 'static> WsManager<T> {

    pub async fn start() -> Self{
        let mut subscriptions = Vec::<Subscription>::new();
        let (new_sub_tx, new_sub_rx) = unbounded_channel::<Control<T>>();
        let (funnel_tx, funnel_rx) = unbounded_channel::<T>();

        let mut funnel_tx = funnel_tx;
       
        tokio::spawn(async move {
            let mut new_sub_receiver = new_sub_rx;
            loop {
                if let Some(v) = new_sub_receiver.recv().await {
                    
                }
            }
        });

        WsManager { sub_tx: new_sub_tx, funnel_rx: funnel_rx }
    }

    pub async fn add_subscription(
        &self, receiver: UnboundedReceiver<T>, handle: JoinHandle<()>,
    ) {
        let _ = self.new_sub_tx.send(receiver);
    }

    pub async fn remove_subscription(&self, id: Uuid) {
        let mut subs = self.subscriptions.lock().await;
        let mut i = 0;
        while i < subs.len() {
            if subs[i].receiver_id == id {
                subs[i].handle.abort();
                subs.remove(i);
                break;
            } else {
                i += 1;
            }
        }
    }
}

pub struct Insider {
    subscriptions: Arc<Mutex<Vec<Subscription>>>,
}