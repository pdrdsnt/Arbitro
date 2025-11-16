#[derive(Clone)]
pub struct Edge {
    pub a: u128,
    pub b: u128,
    pub value: u128,
}

impl Default for Edge {
    fn default() -> Self {
        Self {
            a: Default::default(),
            b: Default::default(),
            value: Default::default(),
        }
    }
}

pub const DEFAULT_EDGE: Edge = Edge {
    a: 0,
    b: 0,
    value: 0,
};
