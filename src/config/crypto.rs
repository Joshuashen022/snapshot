use crate::config::SymbolType;
use anyhow::{anyhow, Result};

#[allow(unused_assignments)]
pub fn set_addr_for_crypto(
    _instrument: &str,
    _limit: Option<i32>,
) -> (Option<String>, Option<String>, Option<String>) {
    let level_depth_address: Option<String> = Some(format!("wss://stream.crypto.com/v2/market",));

    (None, None, level_depth_address)
}

/// Inputs: BTC_USDT / BTC_USDT_SWAP / BTC_USDT_221230_SWAP
/// Crypto output: BTC_USDT / BTCUSD-PERP / (Unsupported)
pub fn validate_symbol_crypto(symbol: &str, limit: Option<i32>) -> Result<SymbolType> {
    let splits = symbol.split("_").collect::<Vec<_>>();
    if splits.len() > 4 || splits.len() < 2 {
        return Err(anyhow!("Unsupported Symbol {} for crypto", symbol));
    }

    let is_contract = { symbol.ends_with("_SWAP") && splits.len() <= 4 };
    let is_spot = !symbol.contains("SWAP");
    let is_contract_coin = { splits.len() == 4 && is_contract };

    fn usdt_usd(usdt: &str) -> &str {
        if usdt == "USDT" {
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
        } else {
            format!("{}_{}", splits[0], splits[1])
        }
    };

    let symbol_inner = if limit.is_some() {
        format!("{}.{}", symbol_in, limit.unwrap())
    } else {
        format!("{}", symbol_in)
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
