use crate::binance::connection::{
    binance_perpetual_coin::BinanceSpotOrderBookPerpetualCoin,
    binance_perpetual_usdt::BinanceSpotOrderBookPerpetualUSDT, binance_spot::BinanceOrderBookSpot,
    BinanceDepthT,
};
use crate::config::{DepthConfig, SymbolType};
use crate::crypto::CryptoDepth;
use crate::{Depth, ExchangeType};
use tokio::sync::mpsc::UnboundedReceiver;

pub enum DepthConnection {
    Binance(Box<dyn BinanceDepthT>),
    Crypto(CryptoDepth),
}

impl DepthConnection {
    pub fn new_with(config: DepthConfig) -> Self {
        match config.exchange_type {
            ExchangeType::Binance => {
                let types = config.symbol_type.clone();
                match config.symbol_type {
                    SymbolType::Spot(_) => {
                        DepthConnection::Binance(Box::new(BinanceOrderBookSpot::new()))
                    }
                    SymbolType::ContractUSDT(_) => {
                        DepthConnection::Binance(Box::new(BinanceSpotOrderBookPerpetualUSDT::new()))
                    }
                    SymbolType::ContractCoin(_) => {
                        DepthConnection::Binance(Box::new(BinanceSpotOrderBookPerpetualCoin::new()))
                    }
                }
            }
            ExchangeType::Crypto => {
                let connection_inner = CryptoDepth::new();
                DepthConnection::Crypto(connection_inner)
            }
        }
    }

    /// 增量深度信息模式，目前仅支持 Binance
    pub fn connect_depth_snapshot(&self, config: DepthConfig) -> UnboundedReceiver<Depth> {
        let (rest_address, depth_address) = config.get_depth_snapshot_addresses();

        match self {
            DepthConnection::Binance(connection) => {
                connection.depth_snapshot(rest_address, depth_address).unwrap()
            }
            DepthConnection::Crypto(_connection) => panic!("Unsupported exchange"),
        }
    }

    /// 有限档深度信息模式，支持 Crypto 以及 Binance
    pub fn connect_depth(&self, config: DepthConfig) -> UnboundedReceiver<Depth> {
        match self {
            DepthConnection::Binance(connection) => connection
                .depth(config.get_depth_addresses())
                .unwrap(),
            DepthConnection::Crypto(connection) => connection.level_depth(config).unwrap(),
        }
    }

    pub fn get_snapshot(&self) -> Option<Depth> {
        match self {
            DepthConnection::Binance(connection) => connection.snapshot(),
            DepthConnection::Crypto(connection) => connection.snapshot(),
        }
    }
}
