// TickerManager new subscribe

use core::panicking::panic;
use tokio::sync::mpsc::UnboundedReceiver;
use crate::{Config, DepthConnection, ExchangeType, match_up, SymbolType};
use crate::api::config::{BinanceDepthConnection, BinanceOrderBookType, Method};
use crate::crypto::CryptoDepthConnector;

#[derive(Clone)]
pub struct TickerManager {
    pub config: Config,
    connection: DepthConnection,
}

impl TickerManager{
    pub fn new() -> Self{

        let config = match_up(exchange, symbol, limit, Method::Ticker);

        let connection = match config.exchange_type {
            ExchangeType::Binance => {
                let types = match config.symbol_type {
                    SymbolType::ContractCoin(_) => BinanceOrderBookType::PrepetualCoin,
                    SymbolType::ContractUSDT(_) => BinanceOrderBookType::PrepetualUSDT,
                    SymbolType::Spot(_) => BinanceOrderBookType::Spot,
                };
                let connection_inner = BinanceDepthConnection::with_type(types);
                DepthConnection::Binance(connection_inner)
            }
            ExchangeType::Crypto => {
                let connection_inner = CryptoDepthConnector::new();
                DepthConnection::Crypto(connection_inner)
            }
        };

        Self { config, connection }
    }

    /// Get snapshot stream
    pub fn subscribe(&self) -> UnboundedReceiver<Ticker> {
        let config = self.config.clone();
        if !config.is_ticker(){
            panic!("Wrong config {:?}", config);
        }

        if config.is_depth() {
            let rest_address = config.rest.expect("rest address is empty");

            let depth_address = config.depth.expect("depth address is empty");

            self.connection
                .clone()
                .connect_depth(rest_address, depth_address)
        } else if config.is_normal() {
            self.connection.clone().connect_depth_level(config)
        } else {
            panic!("Unsupported Config {:?}", config);
        }
    }
}


#[derive(Clone, Debug, Copy)]
pub enum OrderDirection {
    Buy,
    Sell,
}

#[derive(Clone, Debug, Copy)]
pub struct Ticker {
    pub lts: i64,
    pub ts: i64,
    pub price: f64,
    pub amount: f64,
    pub direction: OrderDirection,
    pub id: u64,
}