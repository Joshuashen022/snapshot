pub mod binance_spot;
pub mod binance_perpetual_c;
pub mod binance_perpetual_u;

use crate::format::DepthRow;
use crate::{
    OrderbookSnapshot, Orderbook,
    OrderbookType, ExchangeType
};
use crate::connection::binance_spot::BinanceSpotOrderBookSpot;
use crate::connection::binance_perpetual_u::BinanceSpotOrderBookPerpetualU;
use crate::connection::binance_perpetual_c::BinanceSpotOrderBookPerpetualC;

use serde::Deserialize;
use std::borrow::Cow;
use tracing::{error, info, trace};
use anyhow::Result;
use tokio::sync::mpsc::UnboundedReceiver;
use std::thread::sleep;
use std::time::Duration;

pub enum BinanceOrderBookType{
    Spot,
    PrepetualU,
    PrepetualC,
}


pub enum BinanceConnectionType{
    Spot(BinanceSpotOrderBookSpot),
    PrepetualU(BinanceSpotOrderBookPerpetualU),
    PrepetualC(BinanceSpotOrderBookPerpetualC),
}

impl BinanceConnectionType{
    pub fn new_with_type(types: BinanceOrderBookType) -> Self {
        match types {
            BinanceOrderBookType::Spot => BinanceConnectionType::Spot(BinanceSpotOrderBookSpot::new()),
            BinanceOrderBookType::PrepetualU => BinanceConnectionType::PrepetualU(BinanceSpotOrderBookPerpetualU::new()),
            BinanceOrderBookType::PrepetualC => BinanceConnectionType::PrepetualC(BinanceSpotOrderBookPerpetualC::new()),
        }
    }

    pub fn depth(&self) -> Result<UnboundedReceiver<BinanceSpotOrderBookSnapshot>>{
        match self {
            BinanceConnectionType::Spot(inner) => inner.depth(),
            BinanceConnectionType::PrepetualU(inner) => inner.depth(),
            BinanceConnectionType::PrepetualC(inner) => inner.depth(),
        }
    }

    pub fn level_depth(&self) -> Result<UnboundedReceiver<BinanceSpotOrderBookSnapshot>>{
        match self {
            BinanceConnectionType::Spot(inner) => inner.level_depth(),
            BinanceConnectionType::PrepetualU(inner) => inner.level_depth(),
            BinanceConnectionType::PrepetualC(inner) => inner.level_depth(),
        }
    }

}


#[derive(Deserialize, Debug)]

pub struct BinanceSpotOrderBookSnapshot {
    pub last_update_id: i64,
    pub create_time: i64,
    pub event_time: i64,
    pub bids: Vec<DepthRow>,
    pub asks: Vec<DepthRow>,
}

impl BinanceSpotOrderBookSnapshot{
    /// Compare self with other Order Book
    pub fn if_contains(&self, other: &BinanceSpotOrderBookSnapshot) -> bool {
        let mut contains_bids = true;
        let mut contains_asks = true;
        for bid in &other.bids{
            if !self.bids.contains(bid) {
                contains_bids = false;
                break
            }
        }

        for ask in &other.bids{
            if !self.asks.contains(&ask){
                contains_asks = false;
                break
            }
        }

        contains_bids && contains_asks
    }

    /// Find different `bids` and `asks`,
    /// and return as `(bids, asks)`
    pub fn find_different(&self, other: &BinanceSpotOrderBookSnapshot) -> (Vec<DepthRow>, Vec<DepthRow>) {
        let mut bid_different = Vec::new();
        let mut ask_different = Vec::new();

        for bid in &other.bids{
            if !self.bids.contains(bid) {
                bid_different.push(*bid);
            }
        }

        for ask in &other.bids{
            if !self.asks.contains(&ask){
                ask_different.push(*ask);
            }
        }

        (bid_different, ask_different)
    }
}

impl OrderbookSnapshot for BinanceSpotOrderBookSnapshot {
    
    fn get_bids(&self) -> &Vec<DepthRow> {
        return &self.bids
    }

    fn get_asks(&self) -> &Vec<DepthRow> {
        return &self.asks
    }

    fn get_ts(&self) -> i64 {
        self.event_time
    }

    fn get_symbol(&self) -> Cow<str> {
        todo!()
    }

    fn get_id(&self) -> Cow<str> {
        todo!()
    }
}

impl Orderbook for BinanceConnectionType{
    type SnapShotType = BinanceSpotOrderBookSnapshot;

    fn get_snapshot(&self) -> Self::SnapShotType {
        let mut output:Option<BinanceSpotOrderBookSnapshot> = None;
        loop {
            let result = match self{
                BinanceConnectionType::Spot(inner) => inner.snapshot(),
                BinanceConnectionType::PrepetualU(inner) => inner.snapshot(),
                BinanceConnectionType::PrepetualC(inner) =>  inner.snapshot(),
            };

            match result {
                Some(e) => {
                    output = Some(e);
                    break
                },
                None => {
                    error!("Orderbook lock is busy, try again later");
                    sleep(Duration::from_millis(100));
                    continue
                },
            }

        }

        output.unwrap()
    }
    
    //TODO:: decide a valid type
    fn get_type(&self) -> OrderbookType {
        return OrderbookType::Perpetual
    }

    //TODO:: decide a valid type
    fn get_exchange(&self) -> ExchangeType {
        return ExchangeType::Binance
    }
}