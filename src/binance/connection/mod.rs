pub mod binance_perpetual_c;
pub mod binance_perpetual_u;
pub mod binance_spot;
mod connect;

use crate::binance::connection::{
    binance_perpetual_c::BinanceSpotOrderBookPerpetualC,
    binance_perpetual_u::BinanceSpotOrderBookPerpetualU, binance_spot::BinanceOrderBookSpot,
};
use crate::Depth;
use crate::Quote;

use serde::Deserialize;
// use std::borrow::Cow;
use anyhow::Result;
use tokio::sync::mpsc::UnboundedReceiver;
use tracing::error;
// use std::thread::sleep;
// use std::time::Duration;

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
pub enum BinanceConnectionType {
    Spot(BinanceOrderBookSpot),
    PrepetualUSDT(BinanceSpotOrderBookPerpetualU),
    PrepetualCoin(BinanceSpotOrderBookPerpetualC),
}

impl BinanceConnectionType {
    pub fn with_type(types: BinanceOrderBookType) -> Self {
        match types {
            BinanceOrderBookType::Spot => BinanceConnectionType::Spot(BinanceOrderBookSpot::new()),
            BinanceOrderBookType::PrepetualUSDT => {
                BinanceConnectionType::PrepetualUSDT(BinanceSpotOrderBookPerpetualU::new())
            }
            BinanceOrderBookType::PrepetualCoin => {
                BinanceConnectionType::PrepetualCoin(BinanceSpotOrderBookPerpetualC::new())
            }
        }
    }

    pub fn depth(
        &self,
        rest_address: String,
        depth_address: String,
    ) -> Result<UnboundedReceiver<Depth>> {
        match self {
            BinanceConnectionType::Spot(inner) => inner.depth(rest_address, depth_address),
            BinanceConnectionType::PrepetualUSDT(inner) => inner.depth(rest_address, depth_address),
            BinanceConnectionType::PrepetualCoin(inner) => inner.depth(rest_address, depth_address),
        }
    }

    pub fn level_depth(&self, level_address: String) -> Result<UnboundedReceiver<Depth>> {
        match self {
            BinanceConnectionType::Spot(inner) => inner.level_depth(level_address),
            BinanceConnectionType::PrepetualUSDT(inner) => inner.level_depth(level_address),
            BinanceConnectionType::PrepetualCoin(inner) => inner.level_depth(level_address),
        }
    }

    pub fn snapshot(&self) -> Option<Depth> {
        match self {
            BinanceConnectionType::Spot(inner) => inner.snapshot(),
            BinanceConnectionType::PrepetualUSDT(inner) => inner.snapshot(),
            BinanceConnectionType::PrepetualCoin(inner) => inner.snapshot(),
        }
    }

    #[allow(dead_code)]
    pub fn set_symbol(&mut self, symbol: String) {
        let res = match self {
            BinanceConnectionType::Spot(inner) => inner.set_symbol(symbol),
            BinanceConnectionType::PrepetualUSDT(inner) => inner.set_symbol(symbol),
            BinanceConnectionType::PrepetualCoin(inner) => inner.set_symbol(symbol),
        };

        if let Err(e) = res {
            error!("set symbol error {:?}", e);
        }
    }

    //pub fn snapshot(&self) -> Option<BinanceOrderBookSnapshot>{
}
