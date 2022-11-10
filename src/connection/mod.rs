pub mod binance_spot;
pub mod binance_perpetual_c;
pub mod binance_perpetual_u;

use crate::format::DepthRow;
use crate::{
    OrderbookSnapshot, Orderbook,
    OrderbookType, ExchangeType
};
use crate::connection::binance_spot::BinanceOrderBookSpot;
use crate::connection::binance_perpetual_u::BinanceSpotOrderBookPerpetualU;
use crate::connection::binance_perpetual_c::BinanceSpotOrderBookPerpetualC;

use serde::Deserialize;
use std::borrow::Cow;
use tracing::{debug, error, info, trace};
use anyhow::{anyhow, Result};
use tokio::sync::mpsc::UnboundedReceiver;
use std::thread::sleep;
use std::time::Duration;

#[derive(Clone)]
pub enum Connection{
    Binance(BinanceConnectionType),
    Crypto,
}

pub enum OrderBookType {
    Binance(UnboundedReceiver<BinanceOrderBookSnapshot>),
    Crypto,
}

pub enum OrderBookSnapshotType {
    Binance(BinanceOrderBookSnapshot),
    Crypto,
}

impl Connection {
    pub fn connect_depth(&self, rest_address: String, depth_address: String) -> Result<OrderBookType>{
        match self{
            Connection::Binance(connection) => {
                let receiver = connection.depth(rest_address, depth_address)?;
                Ok(OrderBookType::Binance(receiver))
            }
            Connection::Crypto => Err(anyhow!("Unsupported exchange"))
        }
    }

    pub fn connect_depth_level(&self, level_address: String) -> Result<OrderBookType>{
        match self{
            Connection::Binance(connection) => {
                let receiver = connection.level_depth(level_address)?;
                Ok(OrderBookType::Binance(receiver))
            }
            Connection::Crypto => Err(anyhow!("Unsupported exchange"))
        }
    }

    pub fn get_snapshot(&self)-> Option<OrderBookSnapshotType>{
        match self{
            Connection::Binance(connection) => {
                let receiver = connection.get_snapshot();
                Some(OrderBookSnapshotType::Binance(receiver))
            }
            Connection::Crypto => None
        }
    }
}


pub enum BinanceOrderBookType{
    Spot,
    PrepetualU,
    PrepetualC,
}

#[derive(Deserialize, Debug)]
pub struct BinanceOrderBookSnapshot {
    pub last_update_id: i64,
    pub create_time: i64,
    pub event_time: i64,
    pub bids: Vec<DepthRow>,
    pub asks: Vec<DepthRow>,
}

impl BinanceOrderBookSnapshot {
    /// Compare self with other Order Book
    pub fn if_contains(&self, other: &BinanceOrderBookSnapshot) -> bool {
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
    pub fn find_different(&self, other: &BinanceOrderBookSnapshot) -> (Vec<DepthRow>, Vec<DepthRow>) {
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

impl OrderbookSnapshot for BinanceOrderBookSnapshot {
    
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
    type SnapShotType = BinanceOrderBookSnapshot;

    fn get_snapshot(&self) -> Self::SnapShotType {
        let mut output:Option<BinanceOrderBookSnapshot> = None;
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
                    debug!("Orderbook lock is busy, try again later");
                    sleep(Duration::from_millis(10));
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

#[derive(Clone)]
pub enum BinanceConnectionType{
    Spot(BinanceOrderBookSpot),
    PrepetualU(BinanceSpotOrderBookPerpetualU),
    PrepetualC(BinanceSpotOrderBookPerpetualC),
}

impl BinanceConnectionType{
    pub fn new_with_type(types: BinanceOrderBookType) -> Self {
        match types {
            BinanceOrderBookType::Spot => BinanceConnectionType::Spot(BinanceOrderBookSpot::new()),
            BinanceOrderBookType::PrepetualU => BinanceConnectionType::PrepetualU(BinanceSpotOrderBookPerpetualU::new()),
            BinanceOrderBookType::PrepetualC => BinanceConnectionType::PrepetualC(BinanceSpotOrderBookPerpetualC::new()),
        }
    }

    pub fn depth(&self, rest_address: String, depth_address: String) -> Result<UnboundedReceiver<BinanceOrderBookSnapshot>>{
        match self {
            BinanceConnectionType::Spot(inner) =>
                inner.depth(rest_address, depth_address),
            BinanceConnectionType::PrepetualU(inner) =>
                inner.depth(rest_address, depth_address),
            BinanceConnectionType::PrepetualC(inner) =>
                inner.depth(rest_address, depth_address),
        }
    }

    pub fn level_depth(&self, level_address: String) -> Result<UnboundedReceiver<BinanceOrderBookSnapshot>>{
        match self {
            BinanceConnectionType::Spot(inner) => inner.level_depth(level_address),
            BinanceConnectionType::PrepetualU(inner) => inner.level_depth(level_address),
            BinanceConnectionType::PrepetualC(inner) => inner.level_depth(level_address),
        }
    }

    pub fn snapshot(&self) -> Option<BinanceOrderBookSnapshot>{
        match self {
            BinanceConnectionType::Spot(inner) => inner.snapshot(),
            BinanceConnectionType::PrepetualU(inner) => inner.snapshot(),
            BinanceConnectionType::PrepetualC(inner) => inner.snapshot(),
        }
    }


//pub fn snapshot(&self) -> Option<BinanceOrderBookSnapshot>{
}