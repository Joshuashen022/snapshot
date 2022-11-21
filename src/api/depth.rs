use crate::binance::connection::BinanceDepth;
use crate::crypto::CryptoDepth;
use crate::{get_depth_config_from, DepthConfig, DepthConnection};
use serde::Deserialize;
use std::fmt;
use std::fmt::Debug;
use tokio::sync::mpsc::UnboundedReceiver;

#[derive(Clone)]
pub struct DepthManager {
    pub config: DepthConfig,
    connection: DepthConnection,
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
        if config.is_depth() {
            let rest_address = config.rest_url.expect("rest address is empty");

            let depth_address = config.depth_url.expect("depth address is empty");

            self.connection
                .clone()
                .connect_depth(rest_address, depth_address)
        } else if config.is_normal() {
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
        let config = get_depth_config_from(exchange, symbol, limit);

        assert!(config.is_correct(), "Unsupported config {:?}", config);

        let connection = match config.exchange_type {
            ExchangeType::Binance => {
                let types = config.symbol_type.clone();
                let connection_inner = BinanceDepth::with_type(types);
                DepthConnection::Binance(connection_inner)
            }
            ExchangeType::Crypto => {
                let connection_inner = CryptoDepth::new();
                DepthConnection::Crypto(connection_inner)
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
