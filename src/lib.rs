
use tokio::sync::mpsc::UnboundedReceiver;
use anyhow::{Result, Error, anyhow};
use serde::Deserialize;
use tracing::error;


pub mod binance;
pub mod crypto;
mod match_up;


pub use binance::connection::{
    BinanceOrderBookType, BinanceConnectionType, Connection,
    OrderBookReceiver, BinanceOrderBookSnapshot
};

use binance::format::Quote;
use match_up::{match_up, Config, SymbolType};

// pub fn subscribe_depth_snapshot<T: Orderbook>(exchange: &str, symbol: &str, limit: i32)
//                                               -> Result<UnboundedReceiver<T>>

#[derive(Clone)]
pub struct QuotationManager{
    pub config: Config,
    connection: Connection,
}

pub enum OrderBookSnapshot {
    Binance(BinanceOrderBookSnapshot),
    Crypto,
}

impl QuotationManager{

    /// One-time snapshot
    pub fn new(exchange: &str, symbol: &str) -> Result<Self>{
        Self::new_from(exchange, symbol, None)
    }

    /// Keep-updating snapshot
    pub fn new_with_snapshot(exchange: &str, symbol: &str, limit: i32) -> Result<Self>{
        Self::new_from(exchange, symbol, Some(limit))
    }

    // snapshot stream
    pub fn subscribe_depth(&self) -> Result<UnboundedReceiver<Depth>> {
        let config = self.config.clone();
        if config.is_depth(){
            let rest_address = config.rest.
                ok_or(Error::msg("rest address is empty"))?;
            let depth_address = config.depth
                .ok_or(Error::msg("depth address is empty"))?;
            let connection = self.connection.clone();
            connection.connect_depth(rest_address, depth_address)

        } else if config.is_normal() {
            let level_address = config.level_depth
                .ok_or(Error::msg("level address is empty"))?;
            let connection = self.connection.clone();
            connection.connect_depth_level(level_address)

        } else{
            error!("Unsupported Config {:?}", config);
            Err(anyhow!("Unsupported Config"))
        }


    }

    // One single snapshot
    pub fn latest_depth(&self) -> Option<Depth> {
        let connection = self.connection.clone();
        connection.get_snapshot()
    }

    fn new_from(exchange: &str, symbol: &str, limit: Option<i32>) -> Result<Self>{
        let config = match_up(exchange, symbol, limit)?;

        let types = match config.symbol_type{
            SymbolType::ContractC(_) => BinanceOrderBookType::PrepetualC,
            SymbolType::ContractU(_) => BinanceOrderBookType::PrepetualU,
            SymbolType::Spot(_) => BinanceOrderBookType::Spot,
        };

        let connection_inner = BinanceConnectionType::new_with_type(types);

        let connection = match config.exchange_type{
            ExchangeType::Binance => Connection::Binance(connection_inner),
            ExchangeType::Crypto => Connection::Crypto,
        };

        Ok(Self{ config, connection})
    }

}
#[allow(dead_code)]
#[derive(Deserialize, Debug, Clone)]
pub struct Depth{
    /// Send time from Exchange,
    /// if not have, use receive time
    pub ts: i64,
    /// Receive time
    pub lts: i64,
    /// last_update_id
    pub id: i64,
    asks: Vec<Quote>,
    bids: Vec<Quote>,

}
#[allow(dead_code)]
impl Depth{
    fn from_snapshot(orderbook: OrderBookSnapshot) -> Option<Self>{
        match orderbook{
            OrderBookSnapshot::Binance(_) => {},
            OrderBookSnapshot::Crypto => (),
        }
        None
    }
}

/// 行情类型: 现货、永续合约
pub enum OrderbookType {
    Spot,
    Perpetual,
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
    fn spot_receiver_works() {
        use tokio::runtime::Runtime;
        
        use crate::connection::BinanceOrderBookType;
        use crate::connection::BinanceConnectionType;

        Runtime::new().unwrap().block_on(async {

            let spot = BinanceConnectionType::new_with_type(BinanceOrderBookType::Spot);
            let mut spot_rx_d= spot.depth().unwrap();
            let mut spot_rx_ld= spot.level_depth().unwrap();

            assert!(spot_rx_d.recv().await.is_some(),"spot.depth!");
            assert!(spot_rx_ld.recv().await.is_some(), "spot.level_depth!");

        });
                
    }
    #[test]
    fn contract_u_receiver_works() {
        // Can't pass, but example works
        use tokio::runtime::Runtime;
        
        use crate::connection::BinanceOrderBookType;
        use crate::connection::BinanceConnectionType;

        Runtime::new().unwrap().block_on(async {
            let contract_u = BinanceConnectionType::new_with_type(BinanceOrderBookType::PrepetualU);

            let mut con_u_rx_d = contract_u.depth().unwrap();
            let mut con_u_rx_ld = contract_u.level_depth().unwrap();

            assert!(con_u_rx_d.recv().await.is_some(), "contract_u.depth!");
            assert!(con_u_rx_ld.recv().await.is_some(), "contract_u.level_depth!");

        });
                
    }
    #[test]
    fn contract_c_receiver_works() {
        use tokio::runtime::Runtime;
        
        use crate::connection::BinanceOrderBookType;
        use crate::connection::BinanceConnectionType;

        Runtime::new().unwrap().block_on(async {
            let contract_c = BinanceConnectionType::new_with_type(BinanceOrderBookType::PrepetualC);

            let mut con_c_rx_d = contract_c.depth().unwrap();
            let mut con_c_rx_ld = contract_c.level_depth().unwrap();

            assert!(con_c_rx_d.recv().await.is_some(), "contract_c.depth");
            assert!(con_c_rx_ld.recv().await.is_some(), "contract_c.level_depth!");

        });
                
    }
}

//order book 2131063958416, Event 2131063956506-2131063962059(2131063956407)
