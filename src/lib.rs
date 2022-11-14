extern crate core;

use serde::{Deserialize, Serialize};
use tokio::sync::mpsc::UnboundedReceiver;

pub(crate) mod binance;
pub(crate) mod crypto;
mod match_up;

pub use binance::connection::{
    BinanceConnectionType, BinanceOrderBookSnapshot, BinanceOrderBookType,
};
use crypto::CryptoOrderBookSpot;

use match_up::{match_up, Config, Connection, SymbolType};

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
            let level_address = config.level_depth.expect("level address is empty");

            self.connection.clone().connect_depth_level(level_address)
        } else {
            panic!("Unsupported Config {:?}", config);
        }
    }

    /// Get one single snapshot
    pub fn latest_depth(&self) -> Option<Depth> {
        self.connection.clone().get_depth()
    }

    pub fn snapshot(&self) -> Option<BinanceOrderBookSnapshot> {
        self.connection.clone().get_snapshot()
    }

    /// Generate writable String to be stored into local file
    /// `OrderBookStore` serialized String
    pub fn writable(&self) -> Option<String>{
        let transformed = self.snapshot()?.transform_to_local();

        if let Ok(raw) = serde_json::to_string(&transformed){
            Some(format!("{}", raw))
        } else{
            // Unlikely happen
            None
        }
    }

    pub fn new_from(exchange: &str, symbol: &str, limit: Option<i32>) -> Self {
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
#[derive(Serialize, Deserialize, Debug, Clone)]
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

impl Depth{
    /// Compare self with other Order Book
    pub fn if_contains(&self, other: &Depth) -> bool {
        let mut contains_bids = true;
        let mut contains_asks = true;
        for bid in &other.bids {
            if !self.bids.contains(bid) {
                contains_bids = false;
                break;
            }
        }

        for ask in &other.bids {
            if !self.asks.contains(&ask) {
                contains_asks = false;
                break;
            }
        }

        contains_bids && contains_asks
    }

    /// Find different `bids` and `asks`,
    /// and return as `(bids, asks)`
    pub fn find_different(&self, other: &Depth) -> (Vec<Quote>, Vec<Quote>) {
        let mut bid_different = Vec::new();
        let mut ask_different = Vec::new();

        for bid in &other.bids {
            if !self.bids.contains(bid) {
                bid_different.push(*bid);
            }
        }

        for ask in &other.bids {
            if !self.asks.contains(&ask) {
                ask_different.push(*ask);
            }
        }

        (bid_different, ask_different)
    }

    pub fn from_string(data: String) -> Self {
        let raw:Self = serde_json::from_str(&data).unwrap();
        raw
    }
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
    use std::io::Write;
    use anyhow::Result;

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

    #[test]
    fn read_and_compare()-> Result<()>{
        use std::fs::OpenOptions;
        use std::io::Read;
        use crate::Depth;

        let mut reader1 = OpenOptions::new()
            .read(true).open("depth.cache").unwrap();
        let mut reader2 = OpenOptions::new()
            .read(true).open("normal.cache").unwrap();

        let mut buffer1 = String::new();
        let mut buffer2 = String::new();

        reader1.read_to_string(&mut buffer1).unwrap();
        reader2.read_to_string(&mut buffer2).unwrap();

        buffer1.pop();
        buffer2.pop();


        let depths:Vec<Depth> = buffer1.split("\n").collect::<Vec<_>>().iter()
            .map(|&data|{
                Depth::from_string(data.to_string())
            })
            .collect();
        let depth_levels:Vec<Depth> = buffer2.split("\n").collect::<Vec<_>>().iter()
            .map(|&data|{
                Depth::from_string(data.to_string())
            })
            .collect();

        let mut file = OpenOptions::new();
        let mut reader = file.create(true).write(true).open("results").unwrap();
        let message = format!("depths {}, depth_levels {} \n", depths.len(), depth_levels.len());
        reader.write_all(message.as_bytes()).unwrap_or(());

        let mut res_queue = Vec::new();
        for depth_level in &depth_levels{

            let mut matchs_depths = Vec::new();
            let mut differents_len = 200;
            let mut different = Vec::new();
            for depth in &depths{
                let 结果 = depth.if_contains(depth_level);
                let depth_time = depth.ts;
                let depth_id = depth.id;
                // println!(" Depth {}-{} Depth Level {}-{} {}", depth_time, depth_id, dl_time ,dl_id , 结果);

                // println!("Time {} Id {} {}", depth_time - dl_time, depth_id - dl_id, 结果);
                // println!("different bids {} asks {}", different_bids.len(), different_asks.len());
                // println!("bids {} asks {}", level_b_len, level_a_len);
                if 结果 {
                    matchs_depths.push(depth.clone());
                } else {
                    let (bid, ask) = depth.find_different(depth_level);
                    if (bid.len() + ask.len()) < differents_len {
                        differents_len = bid.len() + ask.len();
                        different = vec![(bid, ask, depth_id, depth_time)];
                    }
                }
            }
            let dl_time = depth_level.ts;
            let dl_id = depth_level.id;
            let length = matchs_depths.len();
            let mut res = format!("{} {} {} matches: ", dl_time, dl_id, matchs_depths.len());
            for m in matchs_depths {
                res += &format!("{} ", m.id);
            }
            if length == 0 {
                for (bid, ask, depth_id, depth_time) in different {
                    res += &format!("depth_id {} depth_time {}, bid {}, ask {}", depth_id, depth_time, bid.len(), ask.len());
                }
            }

            res_queue.push(res);
        }

        for raw in res_queue {
            let raw = format!("{}\n", raw);
            reader.write_all(raw.as_bytes()).unwrap_or(());

        }

        Ok(())
    }
}
