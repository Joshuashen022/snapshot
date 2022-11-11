use crate::binance::connection::BinanceOrderBookSnapshot;
use crate::binance::format::{SharedT, EventT, SnapshotT};
use crate::Quote;

use std::collections::btree_map::BTreeMap;
use std::time::{SystemTime, UNIX_EPOCH};
// use std::sync::{Arc, RwLock};
use ordered_float::OrderedFloat;
use serde::Deserialize;
use tracing::{debug, info, warn};
// use anyhow::{Result, anyhow};

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

impl EventT for EventSpot {

    /// [E.U,..,S.u,..,E.u]
    fn matches(&self, snap_shot_id: i64) -> bool {

        debug!("order book {}, Event {}-{}",
            snap_shot_id,
            self.first_update_id,
            self.last_update_id
        );

        self.first_update_id <= snap_shot_id + 1 && snap_shot_id + 1 <= self.last_update_id
    }

    /// [E.U,..,E.u] S.u
    fn behind(&self, snap_shot_id: i64) -> bool{

        debug!("order book {}, Event {}-{}",
            snap_shot_id,
            self.first_update_id,
            self.last_update_id
        );

        self.last_update_id <= snap_shot_id
    }

    /// S.u [E.U,..,E.u]
    fn ahead(&self, snap_shot_id: i64) -> bool{

        debug!("order book {}, Event {}-{}",
            snap_shot_id,
            self.first_update_id,
            self.last_update_id
        );

        self.first_update_id > snap_shot_id + 1
    }

    ///
    fn equals(&self, snap_shot_id: i64) -> bool{
        debug!("order book {}, Event {}-{}",
            snap_shot_id,
            self.first_update_id,
            self.last_update_id
        );
        self.first_update_id == snap_shot_id + 1
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

impl SnapshotT for BinanceSnapshotSpot{
    fn id(&self) -> i64 {
        self.last_update_id
    }

    fn bids(&self) -> &Vec<Quote> {
        &self.bids
    }

    fn asks(&self) -> &Vec<Quote> {
        &self.asks
    }

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

    /// Only used for "LevelEvent"
    pub fn set_level_event(&mut self, level_event: LevelEventSpot, time_stamp: i64) {
        self.asks.clear();
        for ask in level_event.asks {
            self.asks.insert(OrderedFloat(ask.price), ask.amount);
        }

        self.bids.clear();
        for bid in level_event.bids {
            self.bids.insert(OrderedFloat(bid.price), bid.amount);
        }

        self.last_update_id = level_event.last_update_id;
        self.send_time = time_stamp;
        self.receive_time = time_stamp;
    }

    pub fn get_snapshot(&self) -> BinanceOrderBookSnapshot {
        let asks = self
            .asks
            .iter()
            .map(|(price, amount)| Quote {
                price: price.into_inner(),
                amount: *amount,
            })
            .collect();

        let bids = self
            .bids
            .iter()
            .rev()
            .map(|(price, amount)| Quote {
                price: price.into_inner(),
                amount: *amount,
            })
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

}

impl SharedT<EventSpot> for SharedSpot{
    type BinanceSnapshot = BinanceSnapshotSpot;
    /// return last_update_id
    fn id(&self) -> i64 {
        self.last_update_id
    }

    fn load_snapshot(&mut self, snapshot: &BinanceSnapshotSpot) {
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
    fn add_event(&mut self, event: EventSpot) {
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
        self.send_time = event.ts;
        self.receive_time = time.as_millis() as i64;
    }
}

#[test]
fn depth_row() {
    let a = Quote {
        amount: 1.0,
        price: 2.0,
    };
    let b = Quote {
        amount: 1.0,
        price: 2.0,
    };
    assert_eq!(a, b);
}
