use anyhow::{anyhow, Result};
use crate::config::SymbolType;

#[allow(unused_assignments)]
pub fn set_addr_for_binance(
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


/// Inputs are BTC_USDT / BTC_USDT_SWAP / BTC_USDT_221230_SWAP,
/// Binance output: btcusd_221230/ btcusdt/ bnbbtc (lower cases)
pub fn validate_symbol_binance(symbol: &str) -> Result<SymbolType> {
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
