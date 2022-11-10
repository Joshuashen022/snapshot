use std::borrow::Cow;
use tokio::sync::mpsc::UnboundedReceiver;
use anyhow::{Result, Error};
use tracing::{error, info, trace};


pub mod connection;
mod format;
mod match_up;

pub use connection::{
    BinanceOrderBookType, BinanceConnectionType, Connection,
    OrderBookType
};

use match_up::{match_up, Config};
use format::DepthRow;
use crate::connection::{BinanceOrderBookSnapshot, OrderBookSnapshotType};
use crate::match_up::SymbolType;

// pub fn subscribe_depth_snapshot<T: Orderbook>(exchange: &str, symbol: &str, limit: i32)
//                                               -> Result<UnboundedReceiver<T>>


pub struct QuotationManager{
    pub config: Config,
    connection: Connection,
}

impl QuotationManager{

    pub fn new_from(exchange: &str, symbol: &str, limit: Option<i32>) -> Result<Self>{
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

    pub fn subscribe_depth_snapshot(&self) -> Result<OrderBookType>
    {
        let config = self.config.clone();

        let rest_address = config.rest.ok_or(Error::msg("rest address is empty"))?;
        let depth_address = config.depth.ok_or(Error::msg("depth address is empty"))?;
        let types = match config.symbol_type{
            SymbolType::ContractC(_) => BinanceOrderBookType::PrepetualC,
            SymbolType::ContractU(_) => BinanceOrderBookType::PrepetualU,
            SymbolType::Spot(_) => BinanceOrderBookType::Spot,
        };

        let connection = self.connection.clone();

        connection.connect_depth(rest_address, depth_address)
    }

    pub fn get_depth_snapshot(&self) -> Option<OrderBookSnapshotType>
    {
        let config = self.config.clone();

        let rest_address = config.rest?;
        let depth_address = config.depth?;
        let types = match config.symbol_type{
            SymbolType::ContractC(_) => BinanceOrderBookType::PrepetualC,
            SymbolType::ContractU(_) => BinanceOrderBookType::PrepetualU,
            SymbolType::Spot(_) => BinanceOrderBookType::Spot,
        };

        let connection = self.connection.clone();

        connection.get_snapshot()
    }

    pub fn subscribe_depth(&self) -> Result<OrderBookType>
    {
        let config = self.config.clone();

        let level_address = config.level_depth.ok_or(Error::msg("level address is empty"))?;

        let types = match config.symbol_type{
            SymbolType::ContractC(_) => BinanceOrderBookType::PrepetualC,
            SymbolType::ContractU(_) => BinanceOrderBookType::PrepetualU,
            SymbolType::Spot(_) => BinanceOrderBookType::Spot,
        };

        let connection = self.connection.clone();

        connection.connect_depth_level(level_address)
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

/// 对应某个交易所里一个币对的行情订阅，对外提供接口支持获取最新截面数据
pub trait Orderbook {
    type SnapShotType: OrderbookSnapshot;
    fn get_snapshot(&self) -> Self::SnapShotType;
    fn get_type(&self) -> OrderbookType;
    fn get_exchange(&self) -> ExchangeType;
}

/// 被返回的截面数据需要支持的一些方法
pub trait OrderbookSnapshot {
    fn get_bids(&self) -> &Vec<DepthRow>;
    fn get_asks(&self) -> &Vec<DepthRow>;
    fn get_id(&self) -> Cow<str>;
    /// Time recorded in data or receive time
    /// (Linux time in `ms`)
    fn get_ts(&self) -> i64;
    /// BTC_USD_SWAP
    fn get_symbol(&self) -> Cow<str>;
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
