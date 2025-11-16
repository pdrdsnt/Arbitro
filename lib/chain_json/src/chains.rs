use std::{collections::BTreeMap, env::home_dir, path::PathBuf, str::FromStr};

use chain_db::chains_db::ChainsDB;

use crate::{
    chain::ChainJsonInput,
    chain_json_model::{BlockChainsJsonModel, ChainDataJsonModelSmall},
};

#[derive(Debug)]
pub struct ChainsJsonInput {
    pub chains: BTreeMap<u64, ChainJsonInput>,
}

impl Default for ChainsJsonInput {
    fn default() -> Self {
        let chains_models: BlockChainsJsonModel = {
            let mut home = home_dir().unwrap();
            home.push("Arbitro/lib/chains/config/chainsData.json");

            match BlockChainsJsonModel::new(home.to_str().unwrap()) {
                Ok(chains) => {
                    println!("chains created from config chains");
                    chains
                }

                Err(err) => {
                    panic!("deserialization error {}", err)
                }
            }
        };

        Self::from(chains_models)
    }
}

impl From<BlockChainsJsonModel> for ChainsJsonInput {
    fn from(value: BlockChainsJsonModel) -> Self {
        //each chain has its db
        let mut v = BTreeMap::new();
        for chain in value.chains.into_iter() {
            let c = ChainJsonInput::from(chain);
            v.insert(c.id, c);
        }

        Self { chains: v }
    }
}

impl ChainsJsonInput {
    pub async fn get_chain_data(&self, id: u64) -> Option<ChainDataJsonModelSmall> {
        let chain = self.chains.get(&id)?;
        let tokens = chain.tokens.clone();
        let dexes = chain.dexes.clone();
        let mut pools = Vec::new();
        chain.pools.iter().for_each(|z| {
            for p in z.1 {
                if pools.contains(p) {
                    pools.push(p.clone());
                }
            }
        });

        Some(ChainDataJsonModelSmall {
            tokens,
            dexes,
            pools,
        })
    }
}
