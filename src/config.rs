use crate::types::BlockNum;
use config::{ConfigError, Environment};
use serde::Deserialize;
use std::collections::HashMap;

use url::Url;
use web3::types::{Address, H256};

#[derive(Debug, Deserialize, Clone)]
pub struct AppConfig {
    pub chain_rpc: Url,
    pub pool_factory_address: Address,
    pub active_pools_start_block: BlockNum,
    pub log_bulk_size: u64,
    pub checking_blocks: Option<HashMap<Address, BlockNum>>,
    pub swap_topic0: H256,
}

impl AppConfig {
    pub fn new() -> Result<AppConfig, ConfigError> {
        let prefix = env!("CARGO_PKG_NAME").to_ascii_uppercase();
        config::Config::builder()
            .add_source(Environment::with_prefix(&prefix).separator("__"))
            .build()?
            .try_deserialize()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::env;

    #[test]
    fn test_app_config_new_success() {
        let pkg_name = env!("CARGO_PKG_NAME").to_ascii_uppercase();
        env::set_var(format!("{}__CHAIN_RPC", pkg_name), "http://localhost:8545");
        env::set_var(format!("{}__POOL_FACTORY_ADDRESS", pkg_name), "0x0000000000000000000000000000000000000000");
        env::set_var(format!("{}__ACTIVE_POOLS_START_BLOCK", pkg_name), "1");
        env::set_var(format!("{}__LOG_BULK_SIZE", pkg_name), "100");
        env::set_var(
            format!("{}__SWAP_TOPIC0", pkg_name),
            "0xd78ad95fa46c994b6551d0da85fc275fe613ce37657fb8d5e3d130840159d822",
        );
        env::set_var(format!("{}__CHECK_BLOCKS__0x5757371414417b8C6CAad45bAeF941aBc7d3Ab32", pkg_name), "100");
        env::set_var(format!("{}__CHECK_BLOCKS__0x5757371414417b8C6CAad45bAeF941aBc7d3Ab39", pkg_name), "100");

        let result = AppConfig::new();
        println!("{:?}", result);
        assert!(result.is_ok());
    }
}
