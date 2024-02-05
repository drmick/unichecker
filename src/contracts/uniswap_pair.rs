use web3::{
    contract::{Contract, Options},
    transports::Http,
    types::{Address, BlockId, U256},
    Web3,
};

pub struct UniswapPairContract {
    contract: Contract<Http>,
}

impl UniswapPairContract {
    pub fn new(address: Address, web3: Web3<Http>, abi: &[u8]) -> web3::ethabi::Result<Self> {
        let contract = Contract::from_json(web3.eth(), address, abi)?;
        Ok(Self { contract })
    }

    pub async fn get_reserves(&self, block: BlockId) -> web3::contract::Result<(U256, U256)> {
        let (token0_reserves, token1_reserves, _): (U256, U256, U256) = self.contract.query("getReserves", (), None, Options::default(), block).await?;
        Ok((token0_reserves, token1_reserves))
    }

    pub async fn get_token_addresses(&self, block: BlockId) -> web3::contract::Result<(Address, Address)> {
        let token0_address: Address = self.contract.query("token0", (), None, Options::default(), block).await?;
        let token1_address: Address = self.contract.query("token1", (), None, Options::default(), block).await?;
        Ok((token0_address, token1_address))
    }
}
