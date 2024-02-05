use web3::{
    transports::Http,
    types::{Address, Bytes, CallRequest, U256},
    Web3,
};

const ALL_PAIRS_LENGTH_SELECTOR: [u8; 4] = [0x57, 0x4f, 0x2b, 0xa3];
const ALL_PAIRS_SELECTOR: [u8; 4] = [0x1E, 0x3D, 0xD1, 0x8B];

pub struct FactorySwapContract {
    address: Address,
}

impl FactorySwapContract {
    pub fn new(address: Address) -> Self {
        Self { address }
    }

    pub async fn all_pairs_length(&self, client: &Web3<Http>) -> web3::Result<u64> {
        let call_request = CallRequest::builder().data(ALL_PAIRS_LENGTH_SELECTOR.into()).to(self.address).build();
        let pairs_length = client.eth().call(call_request, None).await?;
        Ok(U256::from_big_endian(&pairs_length.0).as_u64())
    }

    pub async fn get_pair_by_index(&self, client: &Web3<Http>, index: u64) -> web3::Result<Address> {
        let index = U256::from(index);
        let mut padded_index = [0u8; 32]; // Create a zero-initialized array of 32 bytes
        index.to_big_endian(&mut padded_index);

        let data = Bytes::from([&ALL_PAIRS_SELECTOR, &padded_index[..]].concat());
        let call_request = CallRequest::builder().data(data).to(self.address).build();

        let result = client.eth().call(call_request, None).await?;
        let pair_address = Address::from_slice(&result.0[12..32]);

        Ok(pair_address)
    }
}
