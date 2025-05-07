use std::collections::HashMap;

use ethers::types::H160;

pub struct ChainGraph {
    pub pools_by_token: HashMap<H160, Vec<(H160, bool)>>,
}