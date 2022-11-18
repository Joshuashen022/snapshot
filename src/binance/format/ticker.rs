use std::time::{SystemTime, UNIX_EPOCH};
use anyhow::{anyhow, Result};
use serde::Deserialize;
use crate::{OrderDirection, Quote, Ticker};

#[derive(Deserialize, Debug)]
pub struct EventTicker {
    #[serde(rename = "e")]
    pub event_type: String,
    #[serde(rename = "E")]
    pub event_time: i64,
    #[serde(rename = "s")]
    pub pair: String,
    #[serde(rename = "t")]
    pub last_update_id: i64,
    #[serde(rename = "p")]
    pub price: String,
    #[serde(rename = "q")]
    pub amount: String,
    #[serde(rename = "b")]
    pub buy_id: i64,
    #[serde(rename = "a")]
    pub sell_id: i64,
    #[serde(rename = "T")]
    pub trade_time: i64,
    /// true => sell
    #[serde(rename = "m")]
    pub direction: bool,
    #[serde(rename = "M")]
    pub other: bool,
}

impl EventTicker{
    pub fn add_timestamp_transform_to_ticks(&self) -> Option<Vec<Ticker>> {
        if let Ok(tick) = self.tick(){
            Some(vec![tick])
        } else {
            None
        }
    }

    fn tick(&self) -> Result<Ticker> {
        let now = SystemTime::now().duration_since(UNIX_EPOCH).unwrap();
        let lts = now.as_millis() as i64;
        let ts = self.trade_time;
        let id = self.last_update_id as u64;
        let direction = if self.direction{
            OrderDirection::Sell
        } else {
            OrderDirection::Buy
        };

        let amount = self.amount.parse::<f64>()?;
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
