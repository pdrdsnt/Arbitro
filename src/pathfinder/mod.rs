pub mod pathfinder {
    use std::{
        collections::{HashMap, HashSet},
        hash::Hash,
        marker::PhantomData,
        ops::Deref,
        sync::Arc,
    };

    use tokio::sync::RwLock;

    pub struct Pathfinder<K, V, H>
    where
        K: Eq + Hash + Clone,                 //key
        V: IntoConnections,                   //can be converted into multiple connections
        V::Item: Heuristic<K, H>, //connection that can be converted into a edge used internally
        H: Eq + Ord + Hash + Default + Clone, //evaluator type
    {
        pub space: HashMap<K, Arc<RwLock<V>>>,
        pub start: K,
        pub target: K,
        pub open: HashSet<V::Item>,
        pub closed: HashSet<V::Item>,
        pub h: PhantomData<H>,
    }

    pub trait Pathfind<K, V, H>
    where
        K: Eq + Hash + Clone,     //key
        V: IntoConnections,       //can be converted into multiple connections
        V::Item: Heuristic<K, H>, //connection that can be converted into a edge used internally
        H: Eq + Ord + Hash + Default + Clone,
    {
        fn get_connections(&mut self, from: K) -> Vec<V::Item>;
        fn evaluate(&self) -> Edge<K, H>;
    }

    impl<K, V, H> Pathfind<K, V, H> for Pathfinder<K, V, H>
    where
        K: Eq + Hash + Clone + Default,     //key
        V: IntoConnections,       //can be converted into multiple connections
        V::Item: Heuristic<K, H>, //connection that can be converted into a edge used internally
        H: Eq + Ord + Hash + Default + Clone,
    {
        fn get_connections(&mut self, from: K) -> Vec<V::Item> {
            let _node = self.space.get(&from).unwrap().clone();
            let mut node = _node.blocking_write();
            let mut edges = node.get_connections();
            edges
        }

        fn evaluate(&self) -> Edge<K, H> {
            let mut edge: Edge<K, H> = Edge {
                i: K::default(),
                a: K::default(),
                b: K::default(),
                h: H::default(),
            };
            for e in self.open.iter() {
                let edg = e.edge();
                if edge.h < edg.h {
                    edge = edg;
                }
            }
            edge
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

    pub trait Heuristic<K: Eq + Hash + Clone, H: Eq + Ord + Hash + Default + Clone> {
        fn get_h(&self) -> H;
        fn edge(&self) -> Edge<K, H>;
    }

    pub trait IntoConnections {
        type Item;

        fn get_connections(&mut self) -> Vec<Self::Item>;
    }
}
