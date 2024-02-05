use serde::{Serialize, Serializer};
use web3::types::{Address, U256};

pub type BlockNum = u64;

#[derive(Serialize, Debug)]
pub struct DexPoolData {
    pub pair_address: Address,
    pub token0_address: Address,
    pub token1_address: Address,
    pub token0_symbol: Option<String>,
    pub token1_symbol: Option<String>,
    #[serde(serialize_with = "serialize_u256")]
    pub token0_reserves: U256,
    #[serde(serialize_with = "serialize_u256")]
    pub token1_reserves: U256,
    #[serde(serialize_with = "serialize_u256")]
    pub token0_reserve_balance_of: U256,
    #[serde(serialize_with = "serialize_u256")]
    pub token1_reserve_balance_of: U256,
    pub block_num: u64,
    pub strange_reserves: bool,
}

fn serialize_u256<S>(value: &U256, serializer: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    let s = format!("{}", value); // Convert U256 to decimal string
    serializer.serialize_str(&s)
}

#[derive(Debug, Clone)]
pub struct DexPool {
    pub pair_address: Address,
    pub token0_address: Address,
    pub token1_address: Address,
}
