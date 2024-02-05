use web3::{
    contract::{Contract, Options},
    transports::Http,
    types::{Address, BlockId, U256},
    Web3,
};

pub struct Erc20Contract {
    pub address: Address,
    contract: Contract<Http>,
}
impl Erc20Contract {
    pub async fn new(address: Address, web3: Web3<Http>, abi: &[u8]) -> web3::ethabi::Result<Self> {
        let contract = Contract::from_json(web3.eth(), address, abi)?;
        Ok(Self { address, contract })
    }

    pub async fn balance_of(&self, owner: Address, block: BlockId) -> web3::contract::Result<U256> {
        self.contract.query("balanceOf", owner, None, Options::default(), block).await
    }

    pub async fn get_symbol(&self, block: BlockId) -> web3::contract::Result<String> {
        self.contract.query("symbol", (), None, Options::default(), block).await
    }
}
