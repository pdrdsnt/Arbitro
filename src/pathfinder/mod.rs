pub mod pathfinder {
    use std::{
        collections::HashMap, hash::Hash, sync::Arc
    };

    use tokio::sync::RwLock;

    pub struct Pathfinder<K, V, H>
    where
        K: Eq + Hash + Clone,
        V: IntoIterator + Clone,
        V::Item: Ord + Clone + AsRef<K> + Heuristic<H> + Into<Edge<K, H>>,
        for<'b> &'b V: IntoIterator<Item = &'b V::Item>,
        H: Eq + Ord + Hash,
    {
        space: Arc<RwLock<HashMap<K, Arc<RwLock<V>>>>>,
        edges: Arc<RwLock<HashMap<K, Edge<K, H>>>>,
        start: K,
        target: K,
        open: Vec<K>,
        closed: Vec<K>,
    }

    pub trait Pathfind<K, V, H>
    where
        K: Eq + Hash + Clone,
        V: IntoIterator + Clone,
        V::Item: Ord + Clone + AsRef<K> + Heuristic<H> + Into<Edge<K, H>>,

        for<'b> &'b V: IntoIterator<Item = &'b V::Item>,
        H: Eq + Ord + Hash,
    {
        async fn get_connections(&mut self, from: K) -> Vec<Edge<K, H>>;
    }

    impl<K, V, H> Pathfind<K, V, H> for Pathfinder<K, V, H>
    where
        K: Eq + Hash + Clone,
        V: IntoIterator + Clone,
        V::Item: Ord + Clone + AsRef<K> + Heuristic<H> + Into<Edge<K, H>>,
        for<'b> &'b V: IntoIterator<Item = &'b V::Item>,
        H: Eq + Ord + Hash,
    {
        async fn get_connections(&mut self, from: K) -> Vec<Edge<K, H>> {
            let try_read_keys = self.space.read().await;
            let connections: Arc<RwLock<V>> = try_read_keys.get(&from).unwrap().clone();

            let try_read_node: tokio::sync::RwLockReadGuard<'_, V> = connections.read().await;
            let mut connected_to: Vec<Edge<K, H>> = Vec::new();
            for c in &*try_read_node {
                let e: Edge<K, H> = (*c).clone().into();
                connected_to.push(e);
            }
            connected_to
        }
    }

    pub struct Edge<K, H>
    where
        K: Eq + Hash + Clone,
        H: Eq + Ord + Hash,
    {
        pub i: K,
        pub a: K,
        pub b: K,
        pub h: H,
    }

    pub trait Heuristic<H: Eq + Ord + Hash> {
        fn get_h(&mut self);
    }
}
