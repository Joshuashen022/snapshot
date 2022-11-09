use crate::format::DepthRow;
use crate::connection::BinanceSpotOrderBookSnapshot;

use std::collections::btree_map::BTreeMap;
// use std::sync::{Arc, RwLock};
use serde::Deserialize;
use ordered_float::OrderedFloat;
use anyhow::{Result, Error, anyhow};


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
    pub bids: Vec<DepthRow>,
    #[serde(rename = "a")]
    pub asks: Vec<DepthRow>,
}

impl EventSpot {
    pub fn match_seq_num(&self, expected_id: &i64) -> bool {
        self.first_update_id == *expected_id
    }

    pub fn match_snapshot(&self, updated_id: i64) -> bool {
        let first = self.first_update_id <= updated_id + 1;
        let second = updated_id + 1 <= self.last_update_id;
        println!("{}, {}", first, second);
        self.first_update_id <= updated_id + 1 && updated_id + 1 <= self.last_update_id
    }
}

#[derive(Deserialize, Debug)]
pub struct LevelEventSpot {
    #[serde(rename = "lastUpdateId")]
    pub last_update_id: i64,

    /// Difference in bids
    #[serde(rename = "bids")]
    pub bids: Vec<DepthRow>,

    /// Difference in asks
    #[serde(rename = "asks")]
    pub asks: Vec<DepthRow>,
}


#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BinanceSnapshotSpot {
    pub last_update_id: i64,
    pub bids: Vec<DepthRow>,
    pub asks: Vec<DepthRow>,
}

pub struct SharedSpot {
    last_update_id: i64,
    time_stamp: i64,
    asks: BTreeMap<OrderedFloat<f64>, f64>,
    bids: BTreeMap<OrderedFloat<f64>, f64>,
}

impl SharedSpot {
    pub fn new() -> Self {
        SharedSpot {
            last_update_id: 0,
            time_stamp: 0,
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
        self.time_stamp = event.ts;
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
        self.time_stamp = time_stamp;
    }

    pub fn get_snapshot(&self) -> BinanceSpotOrderBookSnapshot {
        let asks = self.asks
            .iter()
            .map(|(price, amount)| DepthRow {price: price.into_inner(), amount: *amount})
            .collect();

        let bids = self.bids
            .iter()
            .rev()
            .map(|(price, amount)| DepthRow {price: price.into_inner(), amount: *amount})
            .collect();
        let time_stamp = self.time_stamp;
        BinanceSpotOrderBookSnapshot {
            last_update_id: self.last_update_id,
            create_time: time_stamp,
            event_time: time_stamp,
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
    let a = DepthRow{amount:1.0, price: 2.0};
    let b = DepthRow{amount:1.0, price: 2.0};
    assert_eq!(a, b);
}