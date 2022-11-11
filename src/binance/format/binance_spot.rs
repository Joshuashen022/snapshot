use crate::binance::format::Quote;
use crate::binance::connection::BinanceOrderBookSnapshot;

use std::collections::btree_map::BTreeMap;
use std::time::{SystemTime, UNIX_EPOCH};
// use std::sync::{Arc, RwLock};
use serde::Deserialize;
use ordered_float::OrderedFloat;
use anyhow::{Result, anyhow};


#[derive(Deserialize, Debug)]
pub struct EventSpot {
    #[serde(rename = "e")]
    pub ttype: String,
    #[serde(rename = "E")]
    pub ts: i64,
    #[serde(rename = "s")]
    pub pair: String,
    #[serde(rename = "U")]
    pub first_update_id: i64,
    #[serde(rename = "u")]
    pub last_update_id: i64,
    #[serde(rename = "b")]
    pub bids: Vec<Quote>,
    #[serde(rename = "a")]
    pub asks: Vec<Quote>,
}

impl EventSpot {
    pub fn match_seq_num(&self, expected_id: &i64) -> bool {
        self.first_update_id == *expected_id
    }

    pub fn match_snapshot(&self, updated_id: i64) -> bool {
        // let first = self.first_update_id <= updated_id + 1;
        // let second = updated_id + 1 <= self.last_update_id;
        // println!("{}, {}", first, second);
        self.first_update_id <= updated_id + 1 && updated_id + 1 <= self.last_update_id
    }
}

#[derive(Deserialize, Debug)]
pub struct LevelEventSpot {
    #[serde(rename = "lastUpdateId")]
    pub last_update_id: i64,

    /// Difference in bids
    #[serde(rename = "bids")]
    pub bids: Vec<Quote>,

    /// Difference in asks
    #[serde(rename = "asks")]
    pub asks: Vec<Quote>,
}


#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BinanceSnapshotSpot {
    pub last_update_id: i64,
    pub bids: Vec<Quote>,
    pub asks: Vec<Quote>,
}

pub struct SharedSpot {
    pub symbol: String,
    last_update_id: i64,
    create_time: i64,
    send_time: i64,
    receive_time: i64,
    asks: BTreeMap<OrderedFloat<f64>, f64>,
    bids: BTreeMap<OrderedFloat<f64>, f64>,
}

impl SharedSpot {
    pub fn new() -> Self {
        SharedSpot {
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
    pub fn id(&self) -> i64{
        self.last_update_id
    }

    pub fn load_snapshot(&mut self, snapshot: &BinanceSnapshotSpot) {
        self.asks.clear();
        for ask in &snapshot.asks {
            self.asks.insert(OrderedFloat(ask.price), ask.amount);
        }

        self.bids.clear();
        for bid in &snapshot.bids {
            self.bids.insert(OrderedFloat(bid.price), bid.amount);
        }

        self.last_update_id = snapshot.last_update_id;
    }

    /// Only used for "Event"
    pub fn add_event(&mut self, event: EventSpot) {
        let time = SystemTime::now().duration_since(UNIX_EPOCH).unwrap();

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

        self.last_update_id = event.last_update_id;
        self.send_time = event.ts;
        self.receive_time = time.as_millis() as i64;
    }

    /// Only used for "LevelEvent"
    pub fn set_level_event(&mut self, level_event: LevelEventSpot, time_stamp: i64){
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

        self.last_update_id = level_event.last_update_id;
        self.send_time = time_stamp;
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

    /// With give event to update snapshot,
    /// if event doesn't satisfy return error
    pub fn update_snapshot(&mut self, event: EventSpot) -> Result<()>  {
        if event.first_update_id != self.last_update_id + 1 {
            Err(anyhow!(
                "Expect event u to be {}, found {}",
                self.last_update_id + 1,
                event.first_update_id
            ))
        } else{
            self.add_event(event);
            Ok(())
        }

    }
}

#[test]
fn depth_row(){
    let a = Quote {amount:1.0, price: 2.0};
    let b = Quote {amount:1.0, price: 2.0};
    assert_eq!(a, b);
}