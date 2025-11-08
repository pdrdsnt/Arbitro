use std::collections::{BTreeMap, BinaryHeap, HashMap, HashSet};

use crate::{
    edge::Edge,
    path::{Path, PathHead},
};

pub struct Node {
    id: u128,
    value: u128,
    edges: Vec<Edge>,
    nodes: BTreeMap<u128, Node>,
}

impl Node {
    pub fn new(id: u128, nodes: BTreeMap<u128, Node>, edges: Vec<Edge>, value: u128) -> Self {
        Node {
            value,
            edges,
            nodes,
            id,
        }
    }

    pub fn map(&self, from: &Node) -> HashMap<u128, Path> {
        let mut map = HashMap::new();

        let mut paths = BinaryHeap::<Path>::new();
        from.get_edges()
            .into_iter()
            .for_each(|edge| paths.push(Path::default().add_edge(edge)));

        while let Some(_path) = paths.pop() {
            match _path {
                Path::Head(path_head) => {
                    if let Some(id) = path_head.path.last() {
                        if let Some(current) = self.nodes.get(&id.b) {
                            for edge in current.get_edges() {
                                let p = Path::Head(path_head.clone()).add_edge(edge);
                                paths.push(p);
                            }
                        }
                    }
                }
                Path::Done(path_done) => {
                    if let Some(id) = path_done.path.last() {
                        map.insert(id.b, Path::Done(path_done));
                    }
                }
            }
        }

        map
    }

    pub fn get_edges(&self) -> Vec<Edge> {
        return self.edges.clone();
    }

    pub fn propagate(&self, path: Path) -> Vec<Path> {
        let mut new_paths = Vec::<Path>::new();
        let edges = self.get_edges();

        if let Path::Head(_) = &path {
            for edge in edges {
                let new_path = path.clone();
                let p = new_path.add_edge(edge);
                new_paths.push(p);
                match p {
                    Path::Head(path_head) => new_paths.push(Path::Done(path_head)),
                    Path::Done(path_done) => new_paths.push(Path::Done(path_done)),
                }
            }

            new_paths
        } else {
            return vec![path];
        }
    }
}
