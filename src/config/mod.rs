mod binance;
mod crypto;
mod depth;
mod ticker;
mod configuration;

use crate::binance::{BinanceDepth, BinanceTicker};
use crate::crypto::{CryptoDepth, CryptoTicker};
use crate::{Depth, ExchangeType};

use anyhow::{anyhow, Result};
use tokio::sync::mpsc::UnboundedReceiver;
pub use configuration::Config;
pub use configuration::{SymbolType, Method};
pub use depth::DepthConnection;
pub use ticker::TickerConnection;


use binance::{set_addr_for_binance, validate_symbol_binance};
use crypto::{set_addr_for_crypto, validate_symbol_crypto};



/// Crypto contract should panic
pub fn match_up(exchange: &str, symbol: &str, limit: Option<i32>, method: Method) -> Config {
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
        method
    }
}



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
#[cfg(test)]
mod tests {
    use crate::api::config::SymbolType;
    use crate::config::validate_symbol_binance;
    use crate::config::validate_symbol_crypto;
    use crate::config::SymbolType;
    use crate::config::match_up;
    #[test]
    fn match_up_input_test() {


        assert!(validate_symbol_binance("BTC_USTD_221230_SWAP").is_ok());
        assert!(validate_symbol_binance("BTC_USTD_SWAP").is_ok());
        assert!(validate_symbol_binance("BTC_USTD").is_ok());

        assert!(validate_symbol_crypto("BTC_USTD_221230_SWAP", None).is_err());
        assert!(validate_symbol_crypto("BTC_USTD_SWAP", None).is_ok());
        assert!(validate_symbol_crypto("BTC_USTD", None).is_ok());
    }

    #[test]
    fn valid_symbols() {

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

        let binance_config = match_up("binance", "BTC_USTD_221230_SWAP", Some(1000));

        assert!(binance_config.is_binance());
        assert!(binance_config.is_contract_coin());

        let crypto_config = match_up("crypto", "BTC_USDT", None);

        assert!(crypto_config.is_crypto());
        assert!(crypto_config.is_spot());
        assert!(crypto_config.get_symbol().is_some());

        assert_eq!(
            crypto_config.get_symbol().unwrap(),
            String::from("BTC_USDT.50")
        );

        let crypto_config = match_up("crypto", "BTC_USDT_SWAP", None);
        assert_eq!(
            crypto_config.get_symbol().unwrap(),
            String::from("BTCUSD-PERP.50")
        );

        let crypto_config = match_up("crypto", "BTC_USDT", Some(10));
        assert_eq!(
            crypto_config.get_symbol().unwrap(),
            String::from("BTC_USDT.10")
        );
    }

    #[test]
    #[should_panic]
    fn in_valid_symbol() {
        use crate::api::config::validate_symbol_binance;
        let symbol = "BTC_USTD_221230_SWAP_";
        validate_symbol_binance(symbol).unwrap();
    }

}