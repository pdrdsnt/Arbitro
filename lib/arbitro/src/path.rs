use std::{collections::HashSet, path};

use crate::edge::{self, Edge};

#[derive(Clone)]
pub enum Path {
    Head(PathHead),
    Done(PathDone),
}

impl Path {
    pub fn add_edge(self, edge: Edge) -> Path {
        match self {
            Path::Head(path_calc) => {
                let mut new_path = path_calc.path.clone();

                new_path.0.push(Edge {
                    a: path_calc
                        .path
                        .0
                        .last()
                        .unwrap_or_else(|| &edge::DEFAULT_EDGE)
                        .b,
                    b: edge.b,
                    value: edge.value,
                });

                if path_calc.contains.contains(&edge.b) {
                    let new_closed_path = Path::Done(PathDone {
                        path: new_path,
                        total: path_calc.total + edge.value,
                    });

                    return new_closed_path;
                } else {
                    let mut updated_contains = path_calc.contains.clone();
                    updated_contains.insert(edge.b);
                    let still_open_path = Path::Head(PathHead {
                        path: new_path,
                        total: path_calc.total + edge.value,
                        contains: updated_contains,
                    });
                    return still_open_path;
                }
            }

            Path::Done(_) => return self,
        }
    }
}

impl Default for Path {
    fn default() -> Self {
        Path::Head(PathHead {
            path: vec![],
            total: 0,
            contains: HashSet::new(),
        })
    }
}

impl PartialEq for Path {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::Head(sh), Self::Head(oh)) => sh.total == oh.total,
            (Self::Done(sh), Self::Done(oh)) => sh.total == oh.total,
            _ => false,
        }
    }
}

impl PartialOrd for Path {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        let v;
        match self {
            Path::Head(path_head) => v = path_head.total,
            Path::Done(path_done) => v = path_done.total,
        };
        match other {
            Path::Head(path_head) => v.partial_cmp(&path_head.total),
            Path::Done(path_done) => v.partial_cmp(&path_done.total),
        }
    }
}

impl Eq for Path {}

impl Ord for Path {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        let v;
        match self {
            Path::Head(path_head) => v = path_head.total,
            Path::Done(path_done) => v = path_done.total,
        };
        match other {
            Path::Head(path_head) => v.cmp(&path_head.total),
            Path::Done(path_done) => v.cmp(&path_done.total),
        }
    }
}

#[derive(Clone)]
pub struct PathHead {
    pub path: EdgeSequence,
    pub total: u128,
    pub contains: HashSet<u128>,
}

#[derive(Clone)]
pub struct PathDone {
    pub path: EdgeSequence,
    pub total: u128,
}

#[derive(Clone)]
pub struct EdgeSequence(Vec<Edge>);

impl EdgeSequence {
    pub fn add_edge(&mut self, edge: Edge) {
        self.0.push(edge);
    }
}
