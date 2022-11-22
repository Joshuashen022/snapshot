
use crate::{get_depth_config_from, DepthConfig};
use serde::Deserialize;
use std::fmt;
use std::fmt::Debug;
use anyhow::Result;
use tokio::sync::mpsc::UnboundedReceiver;
use crate::binance::connection::binance_perpetual_coin::BinanceSpotOrderBookPerpetualCoin;
use crate::binance::connection::binance_perpetual_usdt::BinanceSpotOrderBookPerpetualUSDT;
use crate::binance::connection::binance_spot::BinanceOrderBookSpot;
use crate::crypto::CryptoDepth;
use crate::config::SymbolType;

pub struct DepthManager {
    pub config: DepthConfig,
    connection: Box<dyn DepthT>,
}

impl DepthManager {
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
        if config.is_depth_snapshot() {

            self.connection.depth_snapshot(config).unwrap()

        } else if config.is_depth() {

            self.connection.depth(config).unwrap()

        } else {
            panic!("Unsupported Config {:?}", config);
        }
    }

    /// Get one single snapshot
    pub fn latest_depth(&self) -> Option<Depth> {
        self.connection.snapshot()
    }

    fn new_from(exchange: &str, symbol: &str, limit: Option<i32>) -> Self {
        let config = get_depth_config_from(exchange, symbol, limit);

        assert!(config.is_correct(), "Unsupported config {:?}", config);

        let connection: Box<dyn DepthT> = match config.exchange_type {
            ExchangeType::Binance => {
                match config.symbol_type {
                    SymbolType::Spot(_) => {
                        Box::new(BinanceOrderBookSpot::new())
                    }
                    SymbolType::ContractUSDT(_) => {
                        Box::new(BinanceSpotOrderBookPerpetualUSDT::new())
                    }
                    SymbolType::ContractCoin(_) => {
                        Box::new(BinanceSpotOrderBookPerpetualCoin::new())
                    }
                }
            }
            ExchangeType::Crypto => {
                Box::new(CryptoDepth::new())
            }
        };

        Self { config, connection }
    }
}

#[allow(dead_code)]
#[derive(Deserialize, Clone)]
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

impl Debug for Depth {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Depth")
            .field("send_time", &self.ts)
            .field("receive_time", &self.lts)
            .field("asks", &self.asks.len())
            .field("bids", &self.bids.len())
            .field("id", &self.id)
            .finish()
    }
}

#[derive(Debug, PartialEq, Clone, Copy, Default)]
pub struct Quote {
    pub price: f64,
    pub amount: f64,
}

#[derive(Clone, Debug, Copy)]
pub enum ExchangeType {
    Binance,
    Crypto,
}

pub(crate) trait DepthT {
    fn new() -> Self
        where
            Self: Sized;

    fn depth_snapshot(&self, config: DepthConfig) -> Result<UnboundedReceiver<Depth>>;

    fn depth(&self, config: DepthConfig) -> Result<UnboundedReceiver<Depth>>;

    fn snapshot(&self) -> Option<Depth>;
}