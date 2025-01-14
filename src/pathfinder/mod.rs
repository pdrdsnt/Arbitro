mod pathfinder {
    use std::{
        borrow::Borrow, collections::HashMap, fs::read, future::Future, hash::Hash, sync::Arc,
    };

    use tokio::sync::RwLock;

    struct Pathfinder<K, V, H>
    where
        K: Eq + Hash + Clone + Copy,
        V: IntoIterator + Clone + Copy,
        V::Item: Ord + AsRef<K> + AsRef<H> + Heuristic<H> + TryInto<Edge<K, H>>,
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
        K: Eq + Hash + Clone + Copy,
        V: IntoIterator + Clone + Copy,
        V::Item: Ord + AsRef<K> + AsRef<H> + Heuristic<H> + TryInto<Edge<K, H>>,

        for<'b> &'b V: IntoIterator<Item = &'b V::Item>,
        H: Eq + Ord + Hash,
    {
        async fn get_connections(&mut self, from: K) -> Vec<Edge<K, H>>;
    }

    impl<K, V, H> Pathfind<K, V, H> for Pathfinder<K, V, H>
    where
        K: Eq + Hash + Clone + Copy,
        V: IntoIterator + Clone + Copy,
        V::Item: Ord + AsRef<K> + AsRef<H> + Heuristic<H> + Into<Edge<K, H>>,
        for<'b> &'b V: IntoIterator<Item = &'b V::Item>,
        H: Eq + Ord + Hash,
    {
        async fn get_connections(&mut self, from: K) -> Vec<Edge<K, H>> {
            let try_read_keys = self.space.read().await;
            let connections: Arc<RwLock<V>> = try_read_keys.get(&from).unwrap().clone();

            let try_read_node = connections.read().await;
            let mut connected_to: Vec<Edge<K, H>> = Vec::new();
            for c in try_read_node.into_iter() {
                let e: Edge<K, H> = c.into();
                connected_to.push(e);
            }
            connected_to
        }
    }

    pub struct Edge<K, H>
    where
        K: Eq + Hash + Clone + Copy,
        H: Eq + Ord + Hash,
    {
        pub i: K,
        pub a: K,
        pub b: K,
        pub from_a: bool,
        pub h: H,
    }

    impl<K, H> Heuristic<H> for Edge<K, H>
    where
        K: Eq + Hash + Clone + Copy,
        H: Eq + Ord + Hash,
    {
        fn get_h(&mut self) {
            
        }
    }
    pub trait Heuristic<H: Eq + Ord + Hash> {
        fn get_h(&mut self);
    }
}
