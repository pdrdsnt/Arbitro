mod graph {

    use std::{
        collections::HashMap, marker::PhantomData, sync::{Arc, RwLock}
    };
    
    use crate::pathfinder::pathfinder;

    struct Graph<K: PartialEq, V, H>
    where
        K: PartialEq,
        H: PartialEq + PartialOrd,
        V: pathfinder::Heuristic<H>,
    {
        map: Arc<RwLock<HashMap<K, V>>>,
        _phantom: PhantomData<H>,
    }
}

mod pathfinder
{
    use std::{
        collections::HashMap,
        sync::{Arc, RwLock},
    };

    struct pathfinder<K, N>
    where
        K: PartialEq,
    {
        start: K,
        current: K,
        target: K,
        traversed: Vec<K>,
        edge: Edge<K, N>,
        path: Vec<Edge<K, N>>,
    }

    struct Edge<K, T>
    where
        K: PartialEq,
    {
        pub id: K,
        pub a: K,
        pub b: K,
        pub from_a: bool,
        pub heuristic: T,
    }

    pub trait Heuristic<T: PartialEq + PartialOrd> {
        fn get_h() -> T;
    }

    pub trait Pathfind<K, V>
    where
        K: PartialEq,
    {
        async fn pathfind(&mut self, space: Arc<RwLock<HashMap<K, V>>>);
    }

    impl<K, V, N> Pathfind<K, V> for pathfinder<K, N>
    where
        K: PartialEq,
    {
        async fn pathfind(&mut self, space: Arc<RwLock<HashMap<K, V>>>) {
            
            let try_read_keys = space.try_read();
            let read_keys = match try_read_keys {
                Ok(hash_map) => hash_map,
                Err(_) => return,
            };

            for key in read_keys.keys(){
                
            }
           
        }
    }
}
