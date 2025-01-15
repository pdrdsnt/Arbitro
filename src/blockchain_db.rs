use std::{fs::File, io::Read};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
pub struct BlockChainsModel {
    pub chains: Vec<ChainDataModel>,
}

impl BlockChainsModel {
    pub fn new(json_path: &str) -> Result<Self, serde_json::Error> {
        let mut json_file = File::open(json_path).unwrap();

        let mut str_json = String::new();
        json_file.read_to_string(&mut str_json).unwrap();

        let blockchains: Result<BlockChainsModel, serde_json::Error>  = serde_json::from_str(&str_json);
        blockchains
    }
}
#[derive(Clone, Serialize, Deserialize)]
pub struct ChainDataModel {
    pub id: u8,
    pub name: String,
    pub dexes: Vec<DexModel>,
    pub tokens: Vec<TokenModel>,
    pub abis: ABIModel,
    pub providers: Vec<String>,
}
#[derive(Clone, Serialize, Deserialize)]
pub struct DexModel {
    pub dex_name: String,
    pub factory: String,
    pub version: String,
}
#[derive(Clone, Serialize, Deserialize)]
pub struct TokenModel {
    pub name: String,
    pub symbol: String,
    pub address: String,
    pub decimals: u8,
}
#[derive(Clone, Serialize, Deserialize)]
pub struct ABIInputModel {
    #[serde(rename = "internalType")]
    internal_type: String,
    pub name: String,
    #[serde(rename = "type")]
    pub type_field: String,
}
#[derive(Clone, Serialize, Deserialize)]
pub struct ABIOutputModel {
    #[serde(rename = "internalType")]
    internal_type: String,
    pub name: String,
    #[serde(rename = "type")]
    pub type_field: String,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct ABIEntryModel {
    pub name: Option<String>,
    pub inputs: Option<Vec<ABIInputModel>>, // Option to handle cases with missing inputs
    pub outputs: Option<Vec<ABIOutputModel>>, // Option to handle cases with missing outputs
    pub constant: Option<bool>,        // Option to handle missing "constant" field
    pub payable: Option<bool>,         // Option to handle missing "payable" field
    #[serde(rename = "stateMutability")]
    pub state_mutability: Option<String>, // Option to handle missing "stateMutability" field
    #[serde(rename = "type")]
    pub type_field: String,
    // Add other optional fields as needed (e.g., "anonymous", "indexed")
    pub anonymous: Option<bool>,
    pub indexed: Option<bool>,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct ABIModel {
    #[serde(rename = "V2_FACTORY_ABI")]
    pub v2_factory_abi: Vec<ABIEntryModel>,
    #[serde(rename = "V3_FACTORY_ABI")]
    pub v3_factory_abi: Vec<ABIEntryModel>,
    #[serde(rename = "V2_POOL_ABI")]
    pub v2_pool_abi: Vec<ABIEntryModel>,
    #[serde(rename = "V3_POOL_ABI")]
    pub v3_pool_abi: Vec<ABIEntryModel>,
    #[serde(rename = "BEP_20_ABI")]
    pub bep_20_abi: Vec<ABIEntryModel>,
}
