use tokio::sync::mpsc::UnboundedReceiver;
use crate::config::Config;
use crate::binance::BinanceDepth;
use crate::crypto::CryptoDepth;
use crate::Depth;

#[derive(Clone)]
pub enum DepthConnection {
    Binance(BinanceDepth),
    Crypto(CryptoDepth),
}



impl DepthConnection {

    /// 增量深度信息模式，目前仅支持 Binance
    pub fn connect_depth(
        &self,
        rest_address: String,
        depth_address: String,
    ) -> UnboundedReceiver<Depth> {
        match self {
            DepthConnection::Binance(connection) => {
                connection.depth(rest_address, depth_address).unwrap()
            }
            DepthConnection::Crypto(_connection) => panic!("Unsupported exchange"),
        }
    }

    /// 有限档深度信息模式，支持 Crypto 以及 Binance
    pub fn connect_depth_level(&self, config: Config) -> UnboundedReceiver<Depth> {
        match self {
            DepthConnection::Binance(connection) => {
                connection.level_depth(config.level_depth.unwrap()).unwrap()
            }
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
