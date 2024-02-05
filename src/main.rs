#![deny(
    non_ascii_idents,
    non_shorthand_field_patterns,
    no_mangle_generic_items,
    overflowing_literals,
    path_statements,
    unused_comparisons,
    unused_parens,
    while_true,
    trivial_numeric_casts,
    unused_extern_crates,
    unused_import_braces,
    unused_qualifications,
    unused_must_use,
    clippy::unwrap_used
)]

use std::{
    cmp::min,
    collections::{HashMap, HashSet},
};

use std::path::{Path, PathBuf};

use crate::{
    config::AppConfig,
    contracts::{erc20::Erc20Contract, uniswap_pair::UniswapPairContract},
    services::storage::{CacheService, InMemoryCacheServiceImpl, SymbolKey},
    types::{BlockNum, DexPoolData},
};
use anyhow::Context;
use contracts::factory::FactorySwapContract;
use log::{info, warn};
use tokio::{
    fs,
    fs::File,
    io::{AsyncBufReadExt, AsyncWriteExt, BufReader},
};
use web3::{
    transports::Http,
    types::{Address, BlockId, FilterBuilder, H256, U256},
    Web3,
};

mod config;
mod contracts;
mod services;
mod types;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dotenv::dotenv().ok();
    env_logger::init();
    info!("App is starting...");
    let config = AppConfig::new().context("Failed to load config")?;

    let transport = Http::new(config.chain_rpc.as_ref()).context("Failed to parse url")?;
    let web3_client = Web3::new(transport);

    // init storage service
    let mut cache_service = InMemoryCacheServiceImpl::new();

    // get checking blocks for pair/token from config (because the requirement does not specify where to get this data from)
    let checking_blocks = config.checking_blocks.unwrap_or_default();

    // find last chain block number
    let last_chain_block = web3_client.eth().block_number().await?.as_u64();
    info!("Last chain block: {}", last_chain_block);

    let factory_swap_contract = FactorySwapContract::new(config.pool_factory_address);

    let active_dex_pools = match cfg!(debug_assertions) {
        true => {
            // calculated active pools earlier and saved to file
            let path = PathBuf::from("./cache/actual_pools.txt");
            load_addresses(path).await?
        },
        false => {
            let dex_pools = collect_dex_pools(&web3_client, factory_swap_contract).await?;
            info!("Dex pools amount by factory {:?}: {}", config.pool_factory_address, dex_pools.len());

            check_swaps_since_block(
                &web3_client,
                dex_pools,
                config.active_pools_start_block,
                last_chain_block,
                config.log_bulk_size,
                config.swap_topic0,
            )
            .await?
        },
    };

    info!(
        "Active dex pools amount between {} and {} blocks: {}",
        config.active_pools_start_block,
        last_chain_block,
        active_dex_pools.len()
    );

    let pools_data = load_pools_data(&web3_client, active_dex_pools, last_chain_block, &mut cache_service, checking_blocks).await?;
    let filepath = format!("./output/pools_data_{}_{}.json", config.active_pools_start_block, last_chain_block);
    save_pools_data_to_file(&pools_data, &filepath).await?;

    return Ok(());
}

/// Collects all DEX pools from the FactorySwapContract.
///
/// # Arguments
/// * `client` - A reference to the Web3 Http client.
/// * `factory` - The FactorySwapContract instance to query for pools.
///
/// # Returns
/// A Result containing a HashSet of Addresses representing the DEX pools,
/// or an error in case of failure.
async fn collect_dex_pools(client: &Web3<Http>, factory: FactorySwapContract) -> web3::Result<HashSet<Address>> {
    info!("Collecting DEX pools from factory contract...");
    let length = factory.all_pairs_length(client).await?;
    let mut dex_pools = HashSet::new();
    for i in 0..length {
        let pool = factory.get_pair_by_index(client, i).await?;
        dex_pools.insert(pool);
    }
    Ok(dex_pools)
}

/// Loads data for active DEX pools.
///
/// # Arguments
/// * `web3_client` - A reference to the Web3 Http client used for interacting with the blockchain.
/// * `active_dex_pools` - A HashSet of active DEX pool addresses to load data for.
/// * `last_chain_block` - The last block number in the blockchain to consider for fetching the data.
/// * `cache_service` - A mutable reference to an implementation of the CacheService trait, used for caching.
/// * `checking_blocks` - A HashMap where keys are pool addresses and values are block numbers. This specifies the block number to check for each pool.
async fn load_pools_data(
    web3_client: &Web3<Http>,
    active_dex_pools: HashSet<Address>,
    last_chain_block: BlockNum,
    cache_service: &mut impl CacheService,
    checking_blocks: HashMap<Address, BlockNum>,
) -> anyhow::Result<Vec<DexPoolData>> {
    let mut pools_data = vec![];

    let uniswap_v2_pair_abi = include_bytes!("../abi/UniswapV2Pair.abi.json");
    let erc20_abi = include_bytes!("../abi/erc20.abi.json");

    let pool_size = active_dex_pools.len();
    for (index, pool) in active_dex_pools.into_iter().enumerate() {
        let checking_block = *checking_blocks.get(&pool).unwrap_or(&last_chain_block);
        let block_id = BlockId::Number(checking_block.into());

        let uniswap_v2_pair_contract =
            UniswapPairContract::new(pool, web3_client.clone(), uniswap_v2_pair_abi).context("Failed to init UniswapPairContract")?;
        // get token addresses from pair contract
        let (token0_address, token1_address) = uniswap_v2_pair_contract.get_token_addresses(block_id).await?;

        // get reserves from pair contract
        let (token0_reserves, token1_reserves) = uniswap_v2_pair_contract.get_reserves(block_id).await?;

        // create contract for each token
        let token0_contract = Erc20Contract::new(token0_address, web3_client.clone(), erc20_abi).await?;
        let token1_contract = Erc20Contract::new(token1_address, web3_client.clone(), erc20_abi).await?;

        // get token symbols
        let token0_symbol = get_token_symbol(&token0_contract, cache_service, checking_block).await?;
        let token1_symbol = get_token_symbol(&token1_contract, cache_service, checking_block).await?;

        // get token balances for each token by pair address
        let token0_reserve_balance_of = get_token_balance(&token0_contract, &pool, block_id).await?;
        let token1_reserve_balance_of = get_token_balance(&token1_contract, &pool, block_id).await?;

        if (index + 1) % 100 == 0 {
            println!("Processed {} pairs, remaining: {}", index + 1, pool_size - index - 1);
        }

        let pool_data = DexPoolData {
            pair_address: pool,
            token0_address,
            token1_address,
            token0_symbol,
            token1_symbol,
            token0_reserves,
            token1_reserves,
            token0_reserve_balance_of,
            token1_reserve_balance_of,
            block_num: checking_block,
            strange_reserves: token0_reserves != token0_reserve_balance_of || token1_reserves != token1_reserve_balance_of,
        };

        if pool_data.strange_reserves {
            warn!("Strange reserves: {:?}", pool_data);
        }

        pools_data.push(pool_data);
    }
    Ok(pools_data)
}

/// Checks for swap events in specified DEX pools within a given block range.
///
/// # Arguments
/// * `web3_client` - A reference to the Web3 Http client used for interacting with the blockchain.
/// * `pools` - A HashSet of DEX pool addresses to check for swap events.
/// * `from_block` - The starting block number from which to begin checking for swap events.
/// * `last_chain_block` - The last block number in the blockchain to check for swap events.
/// * `block_range_size` - The size of each block range to query in a single request. This helps in paginating requests to manage large data sets.
/// * `swap_topic0` - Swap topic0 in pool contract.
async fn check_swaps_since_block(
    web3_client: &Web3<Http>,
    pools: HashSet<Address>,
    from_block: BlockNum,
    last_chain_block: BlockNum,
    block_range_size: u64,
    swap_topic0: H256,
) -> anyhow::Result<HashSet<Address>> {
    let mut from_block = from_block;
    let mut active_pools = HashSet::new();

    while from_block <= last_chain_block {
        info!("Checking swaps for pools from block {} to {}", from_block, last_chain_block);
        let to_block = min(from_block + block_range_size, last_chain_block);

        let filter = FilterBuilder::default()
            .from_block(from_block.into())
            .to_block(to_block.into())
            .topics(Some(vec![swap_topic0]), None, None, None)
            .build();

        let logs = web3_client.eth().logs(filter).await?;

        for log in logs.iter() {
            // filter locally by pool address because of web3 filter works slow
            if pools.contains(&log.address) {
                active_pools.insert(log.address);
            }
        }
        from_block = to_block + 1;
    }

    Ok(active_pools)
}

/// get token symbol from contract or cache by block
async fn get_token_symbol<S: CacheService>(
    contract: &Erc20Contract,
    cache_service: &mut S,
    checking_block: BlockNum,
) -> web3::contract::Result<Option<String>> {
    // create key for cache by block
    let symbol_key = SymbolKey(checking_block, contract.address);

    // first check cache
    if let Some(symbol) = cache_service.get_token_symbol(&symbol_key) {
        return Ok(Some(symbol));
    }

    // if not found in cache, get from contract
    match contract.get_symbol(BlockId::Number(checking_block.into())).await {
        Ok(res) => {
            // add to cache
            cache_service.add_token_symbol(symbol_key, res.clone());
            Ok(Some(res))
        },
        Err(e) => {
            handle_contract_error(e)?;
            warn!("Failed to get token symbol at contract {:?} by block {}", contract.address, checking_block);
            Ok(None)
        },
    }
}

/// get token balance from contract by pool address
async fn get_token_balance(contract: &Erc20Contract, pool: &Address, checking_block: BlockId) -> anyhow::Result<U256> {
    match contract.balance_of(*pool, checking_block).await {
        Ok(res) => Ok(res),
        Err(e) => {
            handle_contract_error(e)?;
            warn!("Failed to get token balance for address: {:?} at contract {:?}", pool, contract.address);
            Ok(U256::zero())
        },
    }
}

/// handle contract error
fn handle_contract_error(e: web3::contract::Error) -> web3::contract::Result<()> {
    if let web3::contract::Error::Abi(web3::ethabi::Error::InvalidName(_)) = e {
        // if the contract was destroyed, then we decide that the balance is 0 and the symbol is empty
        Ok(())
    } else {
        log::error!("Contract error: {:?}", e);
        Err(e)
    }
}

async fn save_pools_data_to_file(pool_data: &[DexPoolData], file_path: &str) -> anyhow::Result<()> {
    // Create directory if it doesn't exist
    let path = Path::new(file_path);
    if let Some(dir_path) = path.parent() {
        fs::create_dir_all(dir_path).await?;
    }

    // Serialize data to JSON
    let json = serde_json::to_string(pool_data)?;

    // Write to a file
    let mut file = File::create(path).await?;
    file.write_all(json.as_bytes()).await?;

    Ok(())
}

async fn load_addresses(path: PathBuf) -> Result<HashSet<Address>, anyhow::Error> {
    let file = File::open(path).await.context("Failed to open file")?;
    let reader = BufReader::new(file);

    let mut pairs = HashSet::new();
    let mut lines = reader.lines();
    while let Some(line) = lines.next_line().await.context("Failed to get next line in file")? {
        pairs.insert(line.parse().context("Failed to parse line to Address")?);
    }

    Ok(pairs)
}
