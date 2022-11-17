use crate::crypto::CryptoOrderBookSpot;
use crate::{BinanceConnectionType, Depth, ExchangeType};
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
///
/// https://api.crypto.com/v2/public/get-book?instrument_name=BTC_USDT&depth=10
/// https://api.crypto.com/v2/{method}
/// https://uat-api.3ona.co/v2/{method} // Backup
// use std::fmt::format;
use anyhow::{anyhow, Result};
use tokio::sync::mpsc::UnboundedReceiver;

#[derive(Clone, Debug)]
pub struct Config {
    pub rest: Option<String>,
    pub depth: Option<String>,
    pub level_depth: Option<String>,
    pub symbol_type: SymbolType,
    pub exchange_type: ExchangeType,
}

impl Config {
    pub fn is_depth(&self) -> bool {
        self.depth.is_some() && self.rest.is_some() && self.level_depth.is_none()
    }

    pub fn is_normal(&self) -> bool {
        self.level_depth.is_some() && self.depth.is_none() && self.rest.is_none()
    }

    pub fn is_binance(&self) -> bool {
        match self.exchange_type {
            ExchangeType::Binance => true,
            _ => false,
        }
    }

    pub fn is_crypto(&self) -> bool {
        match self.exchange_type {
            ExchangeType::Crypto => true,
            _ => false,
        }
    }

    pub fn is_contract_usdt(&self) -> bool {
        match self.symbol_type {
            SymbolType::ContractUSDT(_) => true,
            _ => false,
        }
    }

    pub fn is_spot(&self) -> bool {
        match self.symbol_type {
            SymbolType::Spot(_) => true,
            _ => false,
        }
    }

    pub fn is_contract_coin(&self) -> bool {
        match self.symbol_type {
            SymbolType::ContractCoin(_) => true,
            _ => false,
        }
    }

    pub fn get_symbol_contract_usdt(&self) -> Option<String>{
        match &self.symbol_type {
            SymbolType::ContractUSDT(symbol) => Some(symbol.clone()),
            _ => None,
        }
    }

    pub fn get_symbol_contract_coin(&self) -> Option<String>{
        match &self.symbol_type {
            SymbolType::ContractCoin(symbol) => Some(symbol.clone()),
            _ => None,
        }
    }

    pub fn get_symbol_spot(&self) -> Option<String>{
        match &self.symbol_type {
            SymbolType::Spot(symbol) => Some(symbol.clone()),
            _ => None,
        }
    }

    /// Specialized for crypto exchange
    pub fn get_channel(&self) -> Result<String>{
        match self.exchange_type{
            ExchangeType::Crypto => {
                if !self.is_spot(){
                    Err(anyhow!("Channel is unsupported for {:?}", self.symbol_type))
                } else{
                    let symbol = self.get_symbol_spot().unwrap();
                    Ok(symbol)
                }
            },
            _ => Err(anyhow!("Channel is unsupported for {:?}", self.exchange_type)),
        }

    }
}

#[derive(Clone, Debug, PartialOrd, PartialEq)]
pub enum SymbolType {
    Spot(String),
    ContractUSDT(String),
    ContractCoin(String),
}

/// Crypto contract should panic
pub fn match_up(exchange: &str, symbol: &str, limit: Option<i32>) -> Config {
    let exchange_type = match exchange {
        "binance" => ExchangeType::Binance,
        "crypto" => ExchangeType::Crypto,
        _ => panic!("Unsupported Exchange {}", exchange),
    };

    let symbol_type = match exchange_type {
        ExchangeType::Binance => validate_symbol_binance(symbol).unwrap(),
        ExchangeType::Crypto => validate_symbol_crypto(symbol, limit).unwrap(),
    };

    let (rest_address, depth_address, level_depth_address) = match exchange_type {
        ExchangeType::Binance => set_addr_for_binance(symbol_type.clone(), limit),
        ExchangeType::Crypto => {
            let symbol = match symbol_type.clone() {
                SymbolType::Spot(s) => s,
                SymbolType::ContractUSDT(s) => s,
                _ => panic!("Crypto is supported for {}", symbol),
            };
            set_addr_for_crypto(&symbol, limit)
        }
    };

    Config {
        rest: rest_address,
        depth: depth_address,
        level_depth: level_depth_address,
        symbol_type,
        exchange_type,
    }
}

/// Inputs: BTC_USDT / BTC_USDT_SWAP(Unsupported) / BTC_USDT_221230_SWAP
/// Crypto output: BTC_USDT.50 / BTCUSD-PERP.50 / (Unsupported)
fn validate_symbol_crypto(symbol: &str, limit: Option<i32>) -> Result<SymbolType> {
    let splits = symbol.split("_").collect::<Vec<_>>();
    if splits.len() > 4 || splits.len() < 2 {
        return Err(anyhow!("Unsupported Symbol {} for crypto", symbol));
    }

    let is_contract = { symbol.ends_with("_SWAP") && splits.len() <= 4 };
    let is_spot = !symbol.contains("SWAP");
    let is_contract_coin = { splits.len() == 4 && is_contract };

    fn usdt_usd(usdt: &str) -> &str{
        if usdt == "USDT"{
            &usdt[..3]
        } else {
            usdt
        }
    }

    let symbol_in = {
        let mut splits = splits;

        if is_contract {
            splits.pop();
            format!("{}{}-PERP", splits[0], usdt_usd(splits[1]))
        }else {
            format!("{}_{}", splits[0], splits[1])
        }
    };

    let symbol_inner = if limit.is_some(){
        format!("{}.{}", symbol_in,  limit.unwrap())
    } else {
        format!("{}.50", symbol_in)
    };

    let result = match (is_contract, is_contract_coin, is_spot) {
        // e.g. "BTCUSD-PERP.50"
        (true, false, false) => SymbolType::ContractUSDT(symbol_inner.to_string()),
        // e.g. "BTC_USDT.50"
        (false, false, true) => SymbolType::Spot(symbol_inner.to_string()),
        _ => return Err(anyhow!("Unsupported Symbol {} for crypto", symbol)),
    };

    Ok(result)
}

/// Inputs are BTC_USDT / BTC_USDT_SWAP / BTC_USDT_221230_SWAP,
/// Binance output: btcusd_221230/ btcusdt/ bnbbtc (lower cases)
fn validate_symbol_binance(symbol: &str) -> Result<SymbolType> {
    let splits = symbol.split("_").collect::<Vec<_>>();
    if splits.len() > 4 || splits.len() < 2 {
        return Err(anyhow!("Unsupported Symbol {} for binance", symbol));
    }

    let is_contract = { symbol.ends_with("_SWAP") && splits.len() <= 4 };
    let is_spot = !symbol.contains("SWAP");
    let is_contract_coin = { splits.len() == 4 && is_contract };

    let symbol_inner = {
        let mut splits = splits;

        if is_contract {
            splits.pop();
        }

        if splits.len() == 3 {
            format!("{}{}_{}", splits[0], splits[1], splits[2]).to_lowercase()
        } else {
            format!("{}{}", splits[0], splits[1]).to_lowercase()
        }
    };

    let result = match (is_contract, is_contract_coin, is_spot) {
        // e.g. "btcusd_221230_swap"
        (true, true, false) => SymbolType::ContractCoin(symbol_inner),
        // e.g. "btcusdt_swap"
        (true, false, false) => SymbolType::ContractUSDT(symbol_inner),
        // e.g. "bnbbtc"
        (false, false, true) => SymbolType::Spot(symbol_inner),
        _ => return Err(anyhow!("Unsupported Symbol {} for binance", symbol)),
    };

    Ok(result)
}

#[allow(unused_assignments)]
fn set_addr_for_binance(
    symbol_type: SymbolType,
    limit: Option<i32>,
) -> (Option<String>, Option<String>, Option<String>) {
    let mut rest_address: Option<String> = None;
    let mut depth_address: Option<String> = None;
    let mut level_depth_address: Option<String> = None;

    if limit.is_some() {
        // Depth Mode, only need `rest_address` and `depth_address`
        let limit = limit.unwrap();
        rest_address = match &symbol_type {
            SymbolType::Spot(inner) => Some(format!(
                "https://api.binance.com/api/v3/depth?symbol={}&limit={}",
                inner.to_uppercase(),
                limit
            )),
            SymbolType::ContractUSDT(inner) => Some(format!(
                "https://fapi.binance.com/fapi/v1/depth?symbol={}&limit={}",
                inner.to_uppercase(),
                limit
            )),
            SymbolType::ContractCoin(inner) => Some(format!(
                "https://dapi.binance.com/dapi/v1/depth?symbol={}&limit={}",
                inner.to_uppercase(),
                limit
            )),
        };

        depth_address = match &symbol_type {
            SymbolType::Spot(inner) => Some(format!(
                "wss://stream.binance.com:9443/ws/{}@depth@100ms",
                inner
            )),
            SymbolType::ContractUSDT(inner) => Some(format!(
                "wss://fstream.binance.com/stream?streams={}@depth@100ms",
                inner
            )),
            SymbolType::ContractCoin(inner) => Some(format!(
                "wss://dstream.binance.com/stream?streams={}@depth@100ms",
                inner
            )),
        };
    } else {
        // Level Mode, only need `level_depth_address`

        level_depth_address = match &symbol_type {
            SymbolType::Spot(inner) => Some(format!(
                "wss://stream.binance.com:9443/ws/{}@depth20@100ms",
                inner
            )),
            SymbolType::ContractUSDT(inner) => Some(format!(
                "wss://fstream.binance.com/stream?streams={}@depth20@100ms",
                inner
            )),
            SymbolType::ContractCoin(inner) => Some(format!(
                "wss://dstream.binance.com/stream?streams={}@depth20@100ms",
                inner
            )),
        };
    }
    (rest_address, depth_address, level_depth_address)
}

#[allow(unused_assignments)]
fn set_addr_for_crypto(
    _instrument: &str,
    _limit: Option<i32>,
) -> (Option<String>, Option<String>, Option<String>) {
    let level_depth_address: Option<String> =
        Some(format!("wss://stream.crypto.com/v2/market", ));

    (None, None, level_depth_address)
}

#[derive(Clone)]
pub enum Connection {
    Binance(BinanceConnectionType),
    Crypto(CryptoOrderBookSpot),
}

impl Connection {
    pub fn connect_depth(
        &self,
        rest_address: String,
        depth_address: String,
    ) -> UnboundedReceiver<Depth> {
        match self {
            Connection::Binance(connection) => {
                connection.depth(rest_address, depth_address).unwrap()
            }
            Connection::Crypto(_connection) => panic!("Unsupported exchange"),
        }
    }

    pub fn connect_depth_level(&self, config: Config) -> UnboundedReceiver<Depth> {
        match self {
            Connection::Binance(connection) => connection.level_depth(config.level_depth.unwrap()).unwrap(),
            Connection::Crypto(connection) => connection.level_depth(config).unwrap(),
        }
    }

    pub fn get_snapshot(&self) -> Option<Depth> {
        match self {
            Connection::Binance(connection) => connection.snapshot(),
            Connection::Crypto(connection) => connection.snapshot(),
        }
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn match_up_input_test() {
        use crate::config::validate_symbol_binance;
        use crate::config::validate_symbol_crypto;

        assert!(validate_symbol_binance("BTC_USTD_221230_SWAP").is_ok());
        assert!(validate_symbol_binance("BTC_USTD_SWAP").is_ok());
        assert!(validate_symbol_binance("BTC_USTD").is_ok());

        assert!(validate_symbol_crypto("BTC_USTD_221230_SWAP", None).is_err());
        assert!(validate_symbol_crypto("BTC_USTD_SWAP", None).is_ok());
        assert!(validate_symbol_crypto("BTC_USTD", None).is_ok());
    }

    #[test]
    fn valid_symbols() {
        use crate::config::validate_symbol_crypto;
        use crate::config::validate_symbol_binance;
        use crate::config::SymbolType;

        assert_eq!(
            SymbolType::ContractCoin(String::from("btcusdt_221230")),
            validate_symbol_binance("BTC_USDT_221230_SWAP").unwrap(),
        );

        assert_eq!(
            SymbolType::ContractUSDT(String::from("btcusdt")),
            validate_symbol_binance("BTC_USDT_SWAP").unwrap()
        );

        assert_eq!(
            SymbolType::Spot(String::from("btcusdt")),
            validate_symbol_binance("BTC_USDT").unwrap()
        );




        assert_eq!(
            SymbolType::ContractUSDT(String::from("BTCUSD-PERP.50")),
            validate_symbol_crypto("BTC_USDT_SWAP", None).unwrap()
        );

        assert_eq!(
            SymbolType::Spot(String::from("BTC_USDT.50")),
            validate_symbol_crypto("BTC_USDT", None).unwrap()
        );

        assert_eq!(
            SymbolType::Spot(String::from("BTC_USDT.10")),
            validate_symbol_crypto("BTC_USDT", Some(10)).unwrap()
        );

    }

    #[test]
    fn config_test() {
        use crate::config::match_up;

        let binance_config = match_up("binance", "BTC_USTD_221230_SWAP", Some(1000));

        assert!(binance_config.is_binance());
        assert!(binance_config.is_contract_coin());

        let crypto_config = match_up("crypto", "BTC_USDT", None);

        assert!(crypto_config.is_crypto());
        assert!(crypto_config.is_spot());
        assert!(crypto_config.get_symbol_spot().is_some());

        assert_eq!(
            crypto_config.get_symbol_spot().unwrap(),
            String::from("BTC_USDT.50")
        );

        let crypto_config = match_up("crypto", "BTC_USDT_SWAP", None);
        assert_eq!(
            crypto_config.get_symbol_contract_usdt().unwrap(),
            String::from("BTCUSD-PERP.50")
        );

        let crypto_config = match_up("crypto", "BTC_USDT", Some(10));
        assert_eq!(
            crypto_config.get_symbol_spot().unwrap(),
            String::from("BTC_USDT.10")
        );
    }

    #[test]
    #[should_panic]
    fn in_valid_symbol1() {
        use crate::config::validate_symbol_binance;
        let symbol = "BTC_USTD_221230_SWAP_";
        validate_symbol_binance(symbol).unwrap();
    }

    #[test]
    #[should_panic]
    fn in_valid_symbol2() {
        use crate::config::validate_symbol_binance;
        let symbol = "BTC_USTD_221230_ABC";
        validate_symbol_binance(symbol).unwrap();
    }

    #[test]
    #[should_panic]
    fn in_valid_symbol3() {
        use crate::config::validate_symbol_binance;
        let symbol = "BTC_USTD_221230SWAP";
        validate_symbol_binance(symbol).unwrap();
    }

    #[test]
    #[should_panic]
    fn in_valid_symbol4() {
        use crate::config::validate_symbol_binance;
        let symbol = "BTC_USTDSWAP";
        validate_symbol_binance(symbol).unwrap();
    }
}
