
use serde::Deserialize;
use tokio::sync::mpsc::UnboundedReceiver;

pub(crate) mod binance;
pub(crate) mod crypto;
mod config;

pub use binance::connection::{
    BinanceConnectionType, BinanceOrderBookSnapshot, BinanceOrderBookType,
};
use crypto::CryptoOrderBookSpot;

use config::{match_up, Config, Connection, SymbolType};

#[derive(Clone)]
pub struct QuotationManager {
    pub config: Config,
    connection: Connection,
}

impl QuotationManager {
    /// Create one-time-20-sized snapshot manager
    pub fn new(exchange: &str, symbol: &str) -> Self {
        Self::new_from(exchange, symbol, None)
    }

    /// Create constant-updating-<limit>-sized snapshot manager
    pub fn with_snapshot(exchange: &str, symbol: &str, limit: i32) -> Self {
        Self::new_from(exchange, symbol, Some(limit))
    }

    /// Get snapshot stream
    pub fn subscribe_depth(&self) -> UnboundedReceiver<Depth> {
        let config = self.config.clone();
        if config.is_depth() {
            let rest_address = config.rest.expect("rest address is empty");

            let depth_address = config.depth.expect("depth address is empty");

            self.connection
                .clone()
                .connect_depth(rest_address, depth_address)
        } else if config.is_normal() {
            let _ = config.clone().level_depth.expect("level address is empty");

            self.connection.clone().connect_depth_level(config)
        } else {
            panic!("Unsupported Config {:?}", config);
        }
    }

    /// Get one single snapshot
    pub fn latest_depth(&self) -> Option<Depth> {
        self.connection.clone().get_snapshot()
    }

    fn new_from(exchange: &str, symbol: &str, limit: Option<i32>) -> Self {
        let config = match_up(exchange, symbol, limit);

        let connection = match config.exchange_type {
            ExchangeType::Binance => {
                let types = match config.symbol_type {
                    SymbolType::ContractCoin(_) => BinanceOrderBookType::PrepetualCoin,
                    SymbolType::ContractUSDT(_) => BinanceOrderBookType::PrepetualUSDT,
                    SymbolType::Spot(_) => BinanceOrderBookType::Spot,
                };
                let connection_inner = BinanceConnectionType::with_type(types);
                Connection::Binance(connection_inner)
            }
            ExchangeType::Crypto => {
                let connection_inner = CryptoOrderBookSpot::new();
                Connection::Crypto(connection_inner)
            }
        };

        Self { config, connection }
    }
}

#[allow(dead_code)]
#[derive(Deserialize, Debug, Clone)]
pub struct Depth {
    /// Send time from Exchange,
    /// if not have, use receive time
    pub ts: i64,
    /// Receive time
    pub lts: i64,
    /// last_update_id
    pub id: i64,
    pub asks: Vec<Quote>,
    pub bids: Vec<Quote>,
}

#[derive(Debug, PartialEq, Clone, Copy, Default)]
pub struct Quote {
    pub price: f64,
    pub amount: f64,
}

/// 交易所类型
#[derive(Clone, Debug, Copy)]
pub enum ExchangeType {
    Binance,
    Crypto,
}

#[cfg(test)]
mod tests {
    #[test]
    fn manager_builder_works() {
        use crate::QuotationManager;

        let exchange = "binance";
        let exchange2 = "crypto";
        let pc_symbol = "btcusd_221230_swap";
        let pu_symbol = "btcusdt_swap";
        let spot_symbol = "bnbbtc";
        let limit = 1000;

        let _ = QuotationManager::with_snapshot(exchange, pc_symbol, limit);
        let _ = QuotationManager::with_snapshot(exchange, pu_symbol, limit);
        let _ = QuotationManager::with_snapshot(exchange, spot_symbol, limit);

        let _ = QuotationManager::new(exchange, pc_symbol);
        let _ = QuotationManager::new(exchange, pu_symbol);
        let _ = QuotationManager::new(exchange, spot_symbol);

        let _ = QuotationManager::with_snapshot(exchange2, pc_symbol, limit);
        let _ = QuotationManager::with_snapshot(exchange2, pu_symbol, limit);
        let _ = QuotationManager::with_snapshot(exchange2, spot_symbol, limit);

        let _ = QuotationManager::new(exchange2, pc_symbol);
        let _ = QuotationManager::new(exchange2, pu_symbol);
        let _ = QuotationManager::new(exchange2, spot_symbol);
    }

    #[test]
    #[should_panic]
    fn manager_builder_wrong_exchange() {
        use crate::QuotationManager;

        let wrong_exchange = "binanc";
        let pc_symbol = "btcusd_221230_swap";
        let limit = 1000;

        let _ = QuotationManager::with_snapshot(wrong_exchange, pc_symbol, limit);
    }

    #[test]
    #[should_panic]
    fn manager_builder_wrong_symbol() {
        use crate::QuotationManager;

        let wrong_exchange = "binance";
        let pc_symbol = "btcusd_221230swap";
        let limit = 1000;

        let _ = QuotationManager::with_snapshot(wrong_exchange, pc_symbol, limit);
    }
}
