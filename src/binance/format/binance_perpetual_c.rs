use crate::binance::format::Quote;
use crate::binance::connection::BinanceOrderBookSnapshot;

use std::collections::BTreeMap;
use std::time::{SystemTime, UNIX_EPOCH};
// use std::sync::{Arc, RwLock};
use serde::Deserialize;
use ordered_float::OrderedFloat;
// use anyhow::{Result, anyhow};

#[derive(Deserialize, Debug)]
pub struct StreamEventPerpetualC {
    pub stream: String,
    pub data: EventPerpetualC,
}

#[derive(Deserialize, Debug)]
pub struct StreamLevelEventPerpetualC {
    pub stream: String,
    pub data: LevelEventPerpetualC,
}

/// 增量深度信息
#[derive(Deserialize, Debug)]
pub struct EventPerpetualC {
    /// Event type
    #[serde(rename = "e")]
    pub ttype: String,

    /// Event time
    #[serde(rename = "E")]
    pub event_time: i64,

    /// Transaction time
    #[serde(rename = "T")]
    pub create_time: i64,

    /// Transaction pair
    #[serde(rename = "s")]
    pub pair: String,

    /// 标的交易对
    #[serde(rename = "ps")]
    pub stander_pair: String,

    /// First `update Id` during the time
    /// between last time update and now
    #[serde(rename = "U")]
    pub first_update_id: i64,

    /// Last `update Id` during the time
    /// between last time update and now
    #[serde(rename = "u")]
    pub last_update_id: i64,

    /// Last `update Id` during the time
    /// between last time update and now
    #[serde(rename = "pu")]
    pub last_message_last_update_id: i64,

    /// Difference in bids
    #[serde(rename = "b")]
    pub bids: Vec<Quote>,

    /// Difference in asks
    #[serde(rename = "a")]
    pub asks: Vec<Quote>,
}

impl EventPerpetualC {
    // pub fn match_seq_num(&self, expected_id: &i64) -> bool {
    //     self.first_update_id == *expected_id
    // }

    /// only for contract_U
    /// Rule: `U<= id <= u`
    pub fn match_snapshot(&self, snapshot_updated_id: i64) -> bool {
        // let first = self.first_update_id <= snapshot_updated_id ;
        // let second = snapshot_updated_id <= self.last_update_id;
        // println!("{}, {}", first, second);
        self.first_update_id <= snapshot_updated_id && snapshot_updated_id <= self.last_update_id
    }
}

/// 有限档深度信息
#[derive(Deserialize, Debug)]
pub struct LevelEventPerpetualC {
    /// Event type
    #[serde(rename = "e")]
    pub ttype: String,

    /// Event time
    #[serde(rename = "E")]
    pub event_time: i64,

    /// create
    #[serde(rename = "T")]
    pub create_time: i64,

    /// Transaction pair
    #[serde(rename = "s")]
    pub pair: String,

    /// 标的交易对
    #[serde(rename = "ps")]
    pub stander_pair: String,

    /// First `update Id` during the time
    /// between last time update and now
    #[serde(rename = "U")]
    pub first_update_id: i64,

    /// Last `update Id` during the time
    /// between last time update and now
    #[serde(rename = "u")]
    pub last_update_id: i64,

    /// Last `update Id` during the time
    /// between last time update and now
    #[serde(rename = "pu")]
    pub last_message_last_update_id: i64,

    /// Difference in bids
    #[serde(rename = "b")]
    pub bids: Vec<Quote>,

    /// Difference in asks
    #[serde(rename = "a")]
    pub asks: Vec<Quote>,
}

#[derive(Deserialize)]
pub struct BinanceSnapshotPerpetualC {

    #[serde(rename = "lastUpdateId")]
    pub last_update_id: i64,

    /// Event time
    #[serde(rename = "E")]
    pub event_time: i64,

    /// Transaction time
    #[serde(rename = "T")]
    pub create_time: i64,

    pub symbol: String,

    pub pair: String,

    pub bids: Vec<Quote>,

    pub asks: Vec<Quote>,
}

pub struct SharedPerpetualC {
    pub symbol: String,
    last_update_id: i64,
    create_time: i64,
    send_time: i64,
    receive_time: i64,
    asks: BTreeMap<OrderedFloat<f64>, f64>,
    bids: BTreeMap<OrderedFloat<f64>, f64>,
}

impl SharedPerpetualC {
    pub fn new() -> Self {
        SharedPerpetualC {
            symbol: String::new(),
            last_update_id: 0,
            create_time: 0,
            send_time: 0,
            receive_time: 0,
            asks: BTreeMap::new(),
            bids: BTreeMap::new(),
        }
    }

    /// return last_update_id
    /// or `u`
    pub fn id(&self) -> i64{
        self.last_update_id
    }

    pub fn load_snapshot(&mut self, snapshot: &BinanceSnapshotPerpetualC) {
        self.asks.clear();
        for ask in &snapshot.asks {
            self.asks.insert(OrderedFloat(ask.price), ask.amount);
        }

        self.bids.clear();
        for bid in &snapshot.bids {
            self.bids.insert(OrderedFloat(bid.price), bid.amount);
        }

        self.last_update_id = snapshot.last_update_id;
        self.send_time = snapshot.event_time;
        self.create_time = snapshot.create_time;
    }

    /// Only used for "Event"
    pub fn add_event(&mut self, event: EventPerpetualC) {

        for ask in event.asks {
            if ask.amount == 0.0 {
                self.asks.remove(&OrderedFloat(ask.price));
            } else {
                self.asks.insert(OrderedFloat(ask.price), ask.amount);
            }
        }

        for bid in event.bids {
            if bid.amount == 0.0 {
                self.bids.remove(&OrderedFloat(bid.price));
            } else {
                self.bids.insert(OrderedFloat(bid.price), bid.amount);
            }
        }
        let time = SystemTime::now().duration_since(UNIX_EPOCH).unwrap();
        self.last_update_id = event.last_update_id;
        self.create_time = event.create_time;
        self.send_time = event.event_time;
        self.receive_time = time.as_millis() as i64;
    }

    /// Only used for "LevelEvent"
    pub fn set_level_event(&mut self, level_event: LevelEventPerpetualC){
        for ask in level_event.asks {
            if ask.amount == 0.0 {
                self.asks.remove(&OrderedFloat(ask.price));
            } else {
                self.asks.insert(OrderedFloat(ask.price), ask.amount);
            }
        }

        for bid in level_event.bids {
            if bid.amount == 0.0 {
                self.bids.remove(&OrderedFloat(bid.price));
            } else {
                self.bids.insert(OrderedFloat(bid.price), bid.amount);
            }
        }
        let time = SystemTime::now().duration_since(UNIX_EPOCH).unwrap();
        self.last_update_id = level_event.last_update_id;
        self.create_time = level_event.create_time;
        self.send_time = level_event.event_time;
        self.receive_time = time.as_millis() as i64;
    }

    pub fn get_snapshot(&self) -> BinanceOrderBookSnapshot {
        let asks = self.asks
            .iter()
            .map(|(price, amount)| Quote {price: price.into_inner(), amount: *amount})
            .collect();

        let bids = self.bids
            .iter()
            .rev()
            .map(|(price, amount)| Quote {price: price.into_inner(), amount: *amount})
            .collect();

        BinanceOrderBookSnapshot {
            symbol: self.symbol.clone(),
            last_update_id: self.last_update_id,
            create_time: self.create_time,
            send_time: self.send_time,
            receive_time: self.receive_time,
            asks,
            bids,
        }
    }

    // With give event to update snapshot,
    // if event doesn't satisfy return error
    // pub fn update_snapshot(&mut self, event: EventPerpetualC) -> Result<()>  {
    //     if event.first_update_id != self.last_update_id + 1 {
    //         Err(anyhow!(
    //             "Expect event u to be {}, found {}",
    //             self.last_update_id + 1,
    //             event.first_update_id
    //         ))
    //     } else{
    //         self.add_event(event);
    //         Ok(())
    //     }
    //
    // }
}

#[test]
fn depth_row(){
    let a = Quote {amount:1.0, price: 2.0};
    let b = Quote {amount:1.0, price: 2.0};
    assert_eq!(a, b);
}