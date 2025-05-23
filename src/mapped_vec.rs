use std::collections::HashMap;

use ethers::types::H160;


/// A container that holds a Vec of (address, value) pairs and a lookup map from address to index.
/// Ensures both structures stay in sync via its API.
#[derive(Clone)]
pub struct MappedVec<V> {
    entries: Vec<(H160, V)>,
    lookup: HashMap<H160, usize>,
}

impl<V> MappedVec<V> {
    /// Creates an empty MappedVec.
    pub fn new() -> Self {
        MappedVec {
            entries: Vec::new(),
            lookup: HashMap::new(),
        }
    }
    pub fn from_array(entries: &[(H160, V)]) -> Self
    where
        V: Clone,
    {
        let mut vec = Vec::with_capacity(entries.len());
        let mut lookup = HashMap::with_capacity(entries.len());
        for (i, (addr, value)) in entries.iter().enumerate() {
            vec.push((*addr, value.clone()));
            lookup.insert(*addr, i);
        }
        MappedVec { entries: vec, lookup }
    }
    /// Inserts a value for the given address.
    /// If the address was already present, replaces the old value and returns it.
    pub fn insert(&mut self, addr: H160, value: V) -> Option<V> {
        if let Some(&idx) = self.lookup.get(&addr) {
            // replace existing
            let (_, old) = std::mem::replace(&mut self.entries[idx], (addr, value));
            Some(old)
        } else {
            // push new
            let idx = self.entries.len();
            self.entries.push((addr, value));
            self.lookup.insert(addr, idx);
            None
        }
    }
    pub fn get_keys(&self) -> Vec<&H160> {
        self.get_keys()
    }
    /// Returns a reference to the value for the given address, if any.
    pub fn get(&self, addr: &H160) -> Option<&V> {
        self.lookup.get(addr).map(|&i| &self.entries[i].1)
    }

    /// Returns a mutable reference to the value for the given address, if any.
    pub fn get_mut(&mut self, addr: &H160) -> Option<&mut V> {
        self.lookup
            .get(addr)
            .and_then(|&i| self.entries.get_mut(i))
            .map(|(_, v)| v)
    }

    /// Remove the value for the given address, if present.
    /// Uses swap_remove to keep the Vec compact and updates the moved element's index.
    pub fn remove(&mut self, addr: &H160) -> Option<V> {
        if let Some(idx) = self.lookup.remove(addr) {
            let (_removed_addr, removed_val) = self.entries.swap_remove(idx);
            // if we swapped another element into idx, update its lookup entry
            if let Some((moved_addr, _)) = self.entries.get(idx) {
                self.lookup.insert(*moved_addr, idx);
            }
            Some(removed_val)
        } else {
            None
        }
    }

    /// Returns an iterator over all stored values.
    pub fn values(&self) -> impl Iterator<Item = &V> {
        self.entries.iter().map(|(_, v)| v)
    }

    /// Returns an iterator over all stored (address, value) pairs.
    pub fn iter(&self) -> impl Iterator<Item = &(H160, V)> {
        self.entries.iter()
    }
}
