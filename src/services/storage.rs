use crate::types::BlockNum;
use std::collections::HashMap;
use web3::types::Address;

#[derive(Eq, Hash, PartialEq, Clone, Debug)]
pub struct SymbolKey(pub BlockNum, pub Address);

pub trait CacheService {
    fn add_token_symbol(&mut self, key: SymbolKey, symbol: String) -> Option<String>;
    fn get_token_symbol(&self, key: &SymbolKey) -> Option<String>;
}

pub struct InMemoryCacheServiceImpl {
    token_symbols: HashMap<SymbolKey, String>,
}

impl InMemoryCacheServiceImpl {
    pub fn new() -> Self {
        InMemoryCacheServiceImpl { token_symbols: HashMap::new() }
    }
}

impl CacheService for InMemoryCacheServiceImpl {
    fn add_token_symbol(&mut self, key: SymbolKey, symbol: String) -> Option<String> {
        self.token_symbols.insert(key, symbol)
    }

    fn get_token_symbol(&self, key: &SymbolKey) -> Option<String> {
        self.token_symbols.get(key).cloned()
    }
}
