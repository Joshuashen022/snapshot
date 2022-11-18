use tokio::sync::mpsc::UnboundedReceiver;
use crate::{Config, DepthConnection, ExchangeType, get_config_from, SymbolType, TickerConnection};
use crate::config::Method;
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

        let config = get_config_from(exchange, symbol, None, Method::Ticker);

        assert!(config.is_correct(), "Unsupported config {:?}", config);

        let connection = match config.exchange_type {
            ExchangeType::Binance =>
                TickerConnection::Binance(BinanceTicker::new()),
            ExchangeType::Crypto =>
                TickerConnection::Crypto(CryptoTicker::new()),
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
            TickerConnection::Binance(connection) =>
                connection.connect(config).unwrap(),
            TickerConnection::Crypto(connection) =>
                connection.connect(config).unwrap(),
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