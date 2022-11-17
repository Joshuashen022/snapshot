pub mod binance_perpetual_coin;
pub mod binance_perpetual_usdt;
pub mod binance_spot;
mod connect;

use crate::binance::connection::{
    binance_perpetual_coin::BinanceSpotOrderBookPerpetualCoin,
    binance_perpetual_usdt::BinanceSpotOrderBookPerpetualUSDT, binance_spot::BinanceOrderBookSpot,
};
use crate::Depth;
use crate::Quote;

use anyhow::Result;
use serde::Deserialize;
use tokio::sync::mpsc::UnboundedReceiver;
use tracing::error;

pub enum BinanceOrderBookType {
    Spot,
    PrepetualUSDT,
    PrepetualCoin,
}

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
#[derive(Clone)]
pub enum BinanceTickerConnection{
    Spot,
    PerpetualUSDT,
    PerpetualCoin,
}

#[derive(Clone)]
pub enum BinanceDepthConnection {
    Spot(BinanceOrderBookSpot),
    PerpetualUSDT(BinanceSpotOrderBookPerpetualUSDT),
    PerpetualCoin(BinanceSpotOrderBookPerpetualCoin),
}

impl BinanceDepthConnection {
    pub fn with_type(types: BinanceOrderBookType) -> Self {
        match types {
            BinanceOrderBookType::Spot => BinanceDepthConnection::Spot(BinanceOrderBookSpot::new()),
            BinanceOrderBookType::PrepetualUSDT => {
                BinanceDepthConnection::PerpetualUSDT(BinanceSpotOrderBookPerpetualUSDT::new())
            }
            BinanceOrderBookType::PrepetualCoin => {
                BinanceDepthConnection::PerpetualCoin(BinanceSpotOrderBookPerpetualCoin::new())
            }
        }
    }

    pub fn depth(
        &self,
        rest_address: String,
        depth_address: String,
    ) -> Result<UnboundedReceiver<Depth>> {
        match self {
            BinanceDepthConnection::Spot(inner) => inner.depth(rest_address, depth_address),
            BinanceDepthConnection::PerpetualUSDT(inner) => inner.depth(rest_address, depth_address),
            BinanceDepthConnection::PerpetualCoin(inner) => inner.depth(rest_address, depth_address),
        }
    }

    pub fn level_depth(&self, level_address: String) -> Result<UnboundedReceiver<Depth>> {
        match self {
            BinanceDepthConnection::Spot(inner) => inner.level_depth(level_address),
            BinanceDepthConnection::PerpetualUSDT(inner) => inner.level_depth(level_address),
            BinanceDepthConnection::PerpetualCoin(inner) => inner.level_depth(level_address),
        }
    }

    pub fn snapshot(&self) -> Option<Depth> {
        match self {
            BinanceDepthConnection::Spot(inner) => inner.snapshot(),
            BinanceDepthConnection::PerpetualUSDT(inner) => inner.snapshot(),
            BinanceDepthConnection::PerpetualCoin(inner) => inner.snapshot(),
        }
    }

    #[allow(dead_code)]
    pub fn set_symbol(&mut self, symbol: String) {
        let res = match self {
            BinanceDepthConnection::Spot(inner) => inner.set_symbol(symbol),
            BinanceDepthConnection::PerpetualUSDT(inner) => inner.set_symbol(symbol),
            BinanceDepthConnection::PerpetualCoin(inner) => inner.set_symbol(symbol),
        };

        if let Err(e) = res {
            error!("set symbol error {:?}", e);
        }
    }
}
