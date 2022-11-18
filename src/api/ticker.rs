// TickerManager new subscribe
use tokio::sync::mpsc::UnboundedReceiver;
use crate::{Config, DepthConnection, ExchangeType, match_up, SymbolType, TickerConnection};
use crate::api::config::Method;
use crate::binance::BinanceTicker;
use crate::binance::connection::BinanceSymbolType;
use crate::crypto::{CryptoDepth, CryptoTicker};
#[derive(Clone)]
pub struct TickerManager {
    pub config: Config,
    connection: TickerConnection,
}

impl TickerManager{
    pub fn new(exchange: &str, symbol: &str) -> Self{

        let limit = Some(50);

        let config = match_up(exchange, symbol, limit, Method::Ticker);

        let connection = match config.exchange_type {
            ExchangeType::Binance => {
                let types = match config.symbol_type {
                    SymbolType::ContractCoin(_) => BinanceSymbolType::PerpetualCoin,
                    SymbolType::ContractUSDT(_) => BinanceSymbolType::PerpetualUSDT,
                    SymbolType::Spot(_) => BinanceSymbolType::Spot,
                };
                let connection_inner = BinanceTicker::new(types);
                TickerConnection::Binance(connection_inner)
            }
            ExchangeType::Crypto => {
                let connection_inner = CryptoTicker::new();
                TickerConnection::Crypto(connection_inner)
            }
        };

        Self { config, connection }
    }

    /// Get snapshot stream
    pub fn subscribe(&self) -> UnboundedReceiver<Vec<Ticker>> {
        let config = self.config.clone();
        if !config.is_ticker(){
            panic!("Wrong config {:?}", config);
        }
        match &self.connection{
            TickerConnection::Binance(_connection) => panic!("Unsupported"),
            TickerConnection::Crypto(connection) => {
                connection.connect(config).unwrap()
            }
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