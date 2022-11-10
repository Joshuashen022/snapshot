pub mod binance_spot;
pub mod binance_perpetual_c;
pub mod binance_perpetual_u;

use crate::format::Quote;
use crate::{
    OrderbookType, ExchangeType,
    OrderBookSnapshot
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

pub enum OrderBookReceiver {
    Binance(UnboundedReceiver<BinanceOrderBookSnapshot>),
    Crypto,
}

impl OrderBookReceiver{
     pub async fn recv(&mut self) -> Option<OrderBookSnapshot> {
         match self {
            OrderBookReceiver::Binance(rx) =>
                Some(OrderBookSnapshot::Binance(rx.recv().await?)),
            _ => None
        }
    }
}




impl Connection {
    pub fn connect_depth(&self, rest_address: String, depth_address: String) -> Result<OrderBookReceiver>{
        match self{
            Connection::Binance(connection) => {
                let receiver = connection.depth(rest_address, depth_address)?;
                Ok(OrderBookReceiver::Binance(receiver))
            }
            Connection::Crypto => Err(anyhow!("Unsupported exchange"))
        }
    }

    pub fn connect_depth_level(&self, level_address: String) -> Result<OrderBookReceiver>{
        match self{
            Connection::Binance(connection) => {
                let receiver = connection.level_depth(level_address)?;
                Ok(OrderBookReceiver::Binance(receiver))
            }
            Connection::Crypto => Err(anyhow!("Unsupported exchange"))
        }
    }

    pub fn get_snapshot(&self)-> Option<OrderBookSnapshot>{
        match self{
            Connection::Binance(connection) => {
                Some(OrderBookSnapshot::Binance(connection.snapshot()?))
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

#[derive(Deserialize, Debug, Clone)]
pub struct BinanceOrderBookSnapshot {
    pub symbol: String,
    pub last_update_id: i64,
    pub create_time: i64,
    pub event_time: i64,
    pub bids: Vec<Quote>,
    pub asks: Vec<Quote>,
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
    pub fn find_different(&self, other: &BinanceOrderBookSnapshot) -> (Vec<Quote>, Vec<Quote>) {
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