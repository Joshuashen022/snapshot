pub mod binance_perpetual_coin;
pub mod binance_perpetual_usdt;
pub mod binance_spot;

mod connect;
mod ticker;

pub use ticker::BinanceTicker;

use crate::Depth;
use crate::Quote;

use anyhow::Result;
use serde::Deserialize;
use tokio::sync::mpsc::UnboundedReceiver;

#[derive(Deserialize, Debug, Clone)]
pub struct BinanceOrderBookSnapshot {
    pub symbol: String,
    pub last_update_id: i64,
    pub create_time: i64,
    pub send_time: i64,
    pub receive_time: i64,
    pub bids: Vec<Quote>,
    pub asks: Vec<Quote>,
}

impl BinanceOrderBookSnapshot {
    /// Compare self with other Order Book
    #[allow(dead_code)]
    pub fn if_contains(&self, other: &BinanceOrderBookSnapshot) -> bool {
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
    #[allow(dead_code)]
    pub fn find_different(&self, other: &BinanceOrderBookSnapshot) -> (Vec<Quote>, Vec<Quote>) {
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

    pub fn depth(&self) -> Depth {
        Depth {
            lts: self.receive_time,
            ts: self.send_time,
            id: self.last_update_id,
            bids: self.bids.clone(),
            asks: self.asks.clone(),
        }
    }
}

pub trait BinanceDepthT {
    fn new() -> Self
    where
        Self: Sized;

    fn depth_snapshot(
        &self,
        rest_address: String,
        depth_address: String,
    ) -> Result<UnboundedReceiver<Depth>>;

    fn depth(&self, level_address: String) -> Result<UnboundedReceiver<Depth>>;

    fn snapshot(&self) -> Option<Depth>;
}
