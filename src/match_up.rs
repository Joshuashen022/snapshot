/// exchange: "binance" / "crypto"
/// symbol: "BTC_USDT" / "FTT_USDT"
///
///
/// // DEPTH CHANNEL
/// subscribe_depth_snapshot(exchange: str, symbol: str, limit: int) -> Channel
///
///
/// // DEPTH SNAPSHOT
/// get_depth_snapshot(exchange: str, symbol: str, limit: int) -> Option<Snapshot>
///
///
/// // LEVEL (default 20)
/// subscribe_depth(exchange: str, symbol: str) -> Channel
///      LEVEL_DEPTH_URL_PC
///      LEVEL_DEPTH_URL_PU
///      LEVEL_DEPTH_URL_SPOT
/// let url = format!("https://api.binance.com/api/v3/depth?symbol={}&limit={}", symbol, limit);
/// btcusd_221230_swap: contract
/// btcusdt_swap: contract
/// bnbbtc: spot
///
/// const DEPTH_URL_PC: &str =      "wss://dstream.binance.com/stream?streams=btcusd_221230@depth@100ms";
///
/// const DEPTH_URL_PU: &str =      "wss://fstream.binance.com/stream?streams=btcusdt@depth@100ms";
///
/// const DEPTH_URL_SPOT: &str =    "wss://stream.binance.com:9443/ws/bnbbtc@depth@100ms";
///
///
/// const LEVEL_DEPTH_URL_PC: &str =    "wss://dstream.binance.com/stream?streams=btcusd_221230@depth20@100ms";
///
/// const LEVEL_DEPTH_URL_PU: &str =    "wss://fstream.binance.com/stream?streams=btcusdt@depth20@100ms";
///
/// const LEVEL_DEPTH_URL_SPOT: &str =  "wss://stream.binance.com:9443/ws/bnbbtc@depth20@100ms";
///
///
/// const REST_PC: &str =   "https://dapi.binance.com/dapi/v1/depth?symbol=BTCUSD_221230&limit=1000";
/// const REST_PU: &str =   "https://fapi.binance.com/fapi/v1/depth?symbol=BTCUSDT&limit=1000";
/// const REST_SPOT: &str = "https://api.binance.com/api/v3/depth?symbol=BNBBTC&limit=1000";

// use std::fmt::format;
use anyhow::{Result, anyhow};

use crate::ExchangeType;

#[derive(Clone, Debug)]
pub struct Config{
    pub rest: Option<String>,
    pub depth: Option<String>,
    pub level_depth: Option<String>,
    pub symbol_type: SymbolType,
    pub exchange_type: ExchangeType,
}

impl Config{

    pub fn is_depth(&self) -> bool {
        self.depth.is_some() && self.rest.is_some() && self.level_depth.is_none()
    }

    pub fn is_normal(&self) -> bool{
        self.level_depth.is_some() &&  self.depth.is_none() && self.rest.is_none()
    }

    pub fn is_binance(&self) -> bool{
        match self.exchange_type {
            ExchangeType::Binance => true,
            _ => false
        }
    }

    pub fn is_crypto(&self) -> bool{
        match self.exchange_type {
            ExchangeType::Crypto => true,
            _ => false
        }
    }
}


#[derive(Clone, Debug)]
pub enum SymbolType{
    Spot(String),
    ContractU(String),
    ContractC(String),
}

pub fn match_up(exchange: &str, symbol: &str, limit: Option<i32>) -> Config{

    let exchange_type = match exchange {
        "binance" => ExchangeType::Binance,
        "crypto" => ExchangeType::Crypto,
        _ => panic!("Unsupported Exchange {}", exchange)
    };

    let symbol_type = validate_symbol(symbol)
        .expect(&format!("Unsupported Symbol {}", symbol));

    let mut rest_address: Option<String> = None;
    let mut depth_address: Option<String> = None;
    let mut level_depth_address: Option<String> = None;

    if limit.is_some(){
        // Depth Mode, only need `rest_address` and `depth_address`
        let limit = limit.unwrap();
        rest_address = match (&symbol_type, exchange_type) {
            (SymbolType::Spot(inner),ExchangeType::Binance)  => {
                Some(format!("https://api.binance.com/api/v3/depth?symbol={}&limit={}", inner.to_uppercase(), limit))
            },
            (SymbolType::ContractU(inner), ExchangeType::Binance) => {
                Some(format!("https://fapi.binance.com/fapi/v1/depth?symbol={}&limit={}", inner.to_uppercase(), limit))
            },
            (SymbolType::ContractC(inner), ExchangeType::Binance) => {
                Some(format!("https://dapi.binance.com/dapi/v1/depth?symbol={}&limit={}", inner.to_uppercase(), limit))
            },
            (SymbolType::Spot(_),ExchangeType::Crypto)  => {
                None
            },
            (SymbolType::ContractU(_), ExchangeType::Crypto) => {
                None
            },
            (SymbolType::ContractC(_), ExchangeType::Crypto) => {
                None
            },
        };

        depth_address = match (&symbol_type, exchange_type) {
            (SymbolType::Spot(inner),ExchangeType::Binance)  => {
                Some(format!("wss://stream.binance.com:9443/ws/{}@depth@100ms", inner))
            },
            (SymbolType::ContractU(inner), ExchangeType::Binance) => {
                Some(format!("wss://fstream.binance.com/stream?streams={}@depth@100ms", inner))
            },
            (SymbolType::ContractC(inner), ExchangeType::Binance) => {
                Some(format!("wss://dstream.binance.com/stream?streams={}@depth@100ms", inner))
            },
            (SymbolType::Spot(_),ExchangeType::Crypto)  => {
                None
            },
            (SymbolType::ContractU(_), ExchangeType::Crypto) => {
                None
            },
            (SymbolType::ContractC(_), ExchangeType::Crypto) => {
                None
            },
        };

    } else {
        // Level Mode, only need `level_depth_address`

        level_depth_address = match (&symbol_type, exchange_type) {
            (SymbolType::Spot(inner),ExchangeType::Binance)  => {
                Some(format!("wss://stream.binance.com:9443/ws/{}@depth20@100ms", inner))
            },
            (SymbolType::ContractU(inner), ExchangeType::Binance) => {
                Some(format!("wss://fstream.binance.com/stream?streams={}@depth20@100ms", inner))
            },
            (SymbolType::ContractC(inner), ExchangeType::Binance) => {
                Some(format!("wss://dstream.binance.com/stream?streams={}@depth20@100ms", inner))
            },
            (SymbolType::Spot(_),ExchangeType::Crypto)  => {
                None
            },
            (SymbolType::ContractU(_), ExchangeType::Crypto) => {
                None
            },
            (SymbolType::ContractC(_), ExchangeType::Crypto) => {
                None
            },
        };

    }

    Config{
        rest: rest_address,
        depth: depth_address,
        level_depth: level_depth_address,
        symbol_type,
        exchange_type
    }
}

/// Valid symbol should be like
/// "btcusd_221230_swap", "btcusdt_swap", "bnbbtc"
fn validate_symbol(symbol: &str) -> Result<SymbolType>{
    if symbol.split("_").collect::<Vec<_>>().len() > 4 {
        return Err(anyhow!("Unsupported Symbol {}", symbol))
    }

    let is_contract = {
        symbol.contains("_swap") && symbol.split("_").collect::<Vec<_>>().len() <= 3
    };
    let is_spot = {
        !symbol.contains("_") && !symbol.contains("swap")
    };
    let is_contract_coin = {
        symbol.split("_").collect::<Vec<_>>().len() == 3 && is_contract
    };

    let symbol_inner = symbol.split("_swap").collect::<Vec<_>>()[0];

    let result = match (is_contract, is_contract_coin, is_spot){
        // e.g. "btcusd_221230_swap"
        (true, true, false) => SymbolType::ContractC(symbol_inner.to_string()),
        // e.g. "btcusdt_swap"
        (true, false, false) => SymbolType::ContractU(symbol_inner.to_string()),
        // e.g. "bnbbtc"
        (false, false, true) => SymbolType::Spot(symbol_inner.to_string()),
        _ => return Err(anyhow!("Unsupported Symbol {}", symbol))
    };

    Ok(result)
}




#[cfg(test)]
mod tests {
    #[test]
    fn match_up_input_test() {
        use crate::match_up::match_up;
        use crate::match_up::validate_symbol;

        assert!(validate_symbol("btcusd_221230_swap").is_ok());
        assert!(validate_symbol("btcusd_swap").is_ok());
        assert!(validate_symbol("btcusd").is_ok());

        let _ = match_up("binance", "btcusd_221230_swap", Some(1000));

        let _ = match_up("binance", "btcusd_swap", Some(1000));

        let _ = match_up("binance", "btcusd", Some(1000));
    }

    #[test]
    #[should_panic]
    fn in_valid_symbol1(){
        use crate::match_up::validate_symbol;
        let symbol = "btcusd_221230_swap_";
        validate_symbol(symbol).unwrap();
    }

    #[test]
    #[should_panic]
    fn in_valid_symbol2(){
        use crate::match_up::validate_symbol;
        let symbol = "btcusd_221230_abc";
        validate_symbol(symbol).unwrap();
    }

    #[test]
    #[should_panic]
    fn in_valid_symbol3(){
        use crate::match_up::validate_symbol;
        let symbol = "btcusd_221230swap";
        validate_symbol(symbol).unwrap();
    }

    #[test]
    #[should_panic]
    fn in_valid_symbol4(){
        use crate::match_up::validate_symbol;
        let symbol = "btcusdswap";
        validate_symbol(symbol).unwrap();
    }
}
