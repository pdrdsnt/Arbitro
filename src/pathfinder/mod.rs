pub mod pathfinder {
    use std::{collections::HashMap, hash::Hash, ops::Deref, sync::Arc};

    use tokio::sync::RwLock;

    pub struct Pathfinder<K, V, H>
    where
        K: Eq + Hash + Clone,
        V: IntoEdges<K, H>,
        V::Item: Clone + Heuristic<H>,
        H: Eq + Ord + Hash,
    {
        pub space: HashMap<K, Arc<RwLock<V>>>,
        pub edges: HashMap<K, Edge<K, H>>,
        pub start: K,
        pub target: K,
        pub open: Vec<K>,
        pub closed: Vec<K>,
    }

    pub trait Pathfind<K, V, H>
    where
        K: Eq + Hash + Clone,
        V: IntoEdges<K, H>,
        V::Item: Clone + Heuristic<H>,
        for<'b> &'b V: IntoIterator<Item = &'b V::Item>,
        H: Eq + Ord + Hash,
    {
        fn get_connections(&mut self, from: K) -> Vec<Edge<K, H>>;
    }

    impl<K, V, H> Pathfinder<K, V, H>
    where
        K: Eq + Hash + Clone,
        V: IntoEdges<K, H> + Copy,
        V::Item: Clone + Heuristic<H>,
        for<'b> &'b V: IntoIterator<Item = &'b V::Item>,
        H: Eq + Ord + Hash,
    {
        fn get_connections(&mut self, from: K) -> Vec<Edge<K, H>> {

            let connections: Arc<RwLock<V>> = self.space.get(&from).unwrap().clone();
            let try_read_node = connections.blocking_read();
            try_read_node.get_edges()
        }
    }

    #[derive(Clone)]
    pub struct Edge<K, H>
    where
        K: Eq + Hash + Clone,
        H: Eq + PartialOrd + Hash,
    {
        pub i: K,
        pub a: K,
        pub b: K,
        pub h: H,
    }

    pub trait Heuristic<H: Eq + Ord + Hash> {
        fn get_h(self) -> H;
    }

    pub trait IntoEdges<K: Eq + Hash + Clone, H: Ord + Eq + PartialOrd + Hash> {
        type Item: Clone + Heuristic<H>;
        fn get_edges(self) -> Vec<Edge<K, H>>;
    }
}
