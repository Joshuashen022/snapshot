use crate::{Depth, OrderDirection, Quote, Ticker};
use anyhow::{anyhow, Result};
use ordered_float::OrderedFloat;
use serde::Deserialize;
use std::collections::BTreeMap;
use std::time::{SystemTime, UNIX_EPOCH};

#[allow(dead_code)]
pub struct TradeShared {
    instrument: String,
    last_update_id: i64,
    send_time: i64,
    receive_time: i64,
    asks: BTreeMap<OrderedFloat<f64>, f64>,
    bids: BTreeMap<OrderedFloat<f64>, f64>,
}

#[allow(dead_code)]
impl TradeShared {
    pub fn new() -> Self {
        TradeShared {
            instrument: String::new(),
            last_update_id: 0,
            send_time: 0,
            receive_time: 0,
            asks: BTreeMap::new(),
            bids: BTreeMap::new(),
        }
    }
}

#[derive(Deserialize, Debug, Clone)]
pub struct TradeEvent {
    pub channel: String,

    pub subscription: String,

    /// Something like "BTC_USDT"
    pub instrument_name: String,

    pub data: Vec<TradeData>,
}

#[derive(Deserialize, Debug, Clone)]
pub struct TradeData {
    #[serde(rename = "s")]
    pub side: String,

    #[serde(rename = "p")]
    pub price: String,

    #[serde(rename = "q")]
    pub quantity: String,

    #[serde(rename = "t")]
    pub trade_time: i64,

    #[serde(rename = "d")]
    pub trade_id: String,

    #[serde(rename = "i")]
    pub instrument_name: String,
}

impl TradeData {
    fn tick(&self) -> Result<Ticker> {
        let now = SystemTime::now().duration_since(UNIX_EPOCH).unwrap();
        let lts = now.as_millis() as i64;
        let ts = self.trade_time;
        let id = self.trade_id.parse::<u64>()?;
        let direction = match self.side.as_str() {
            "BUY" => OrderDirection::Buy,
            "SELL" => OrderDirection::Sell,
            _ => return Err(anyhow!("Unknown direction {}", self.side)),
        };
        let amount = self.quantity.parse::<f64>()?;
        let price = self.price.parse::<f64>()?;

        Ok(Ticker {
            lts,
            ts,
            price,
            amount,
            id,
            direction,
        })
    }
}

impl TradeEvent {
    pub fn add_timestamp_transform_to_ticks(&self) -> Option<Vec<Ticker>> {
        let mut ticks = Vec::new();
        for data in &self.data {
            let tick_res = data.tick();
            if tick_res.is_ok() {
                ticks.push(tick_res.unwrap())
            };
        }

        if ticks.len() > 0 {
            Some(ticks)
        } else {
            None
        }
    }
}
