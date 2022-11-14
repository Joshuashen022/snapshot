use ordered_float::OrderedFloat;
use std::collections::BTreeMap;
use serde::Deserialize;
use crate::Quote;

pub struct Shared {
    pub instrument: String,
    last_update_id: i64,
    create_time: i64,
    send_time: i64,
    receive_time: i64,
    asks: BTreeMap<OrderedFloat<f64>, f64>,
    bids: BTreeMap<OrderedFloat<f64>, f64>,
}

impl Shared {
    pub fn new() -> Self {
        Shared {
            instrument: String::new(),
            last_update_id: 0,
            create_time: 0,
            send_time: 0,
            receive_time: 0,
            asks: BTreeMap::new(),
            bids: BTreeMap::new(),
        }
    }
}
#[derive(Deserialize, Debug)]
pub struct LevelEventStream{
    pub code: i64,
    pub method: String,
    pub result: Event,
}

#[derive(Deserialize, Debug)]
pub struct Event{
    pub depth: i64,
    pub data: Data,
    pub instrument_name: String
}

#[derive(Deserialize, Debug)]
pub struct Data{
    pub bids: Vec<[String;3]>,
    pub asks: Vec<[String;3]>,
}
