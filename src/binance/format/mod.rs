pub mod binance_perpetual_c;
pub mod binance_perpetual_u;
pub mod binance_spot;

use serde::{de::SeqAccess, de::Visitor, Deserialize, Deserializer};
use std::fmt;

use crate::Quote;

impl<'de> Deserialize<'de> for Quote {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        deserializer.deserialize_tuple(2, QuoteVisitor)
    }
}

struct QuoteVisitor;

impl<'de> Visitor<'de> for QuoteVisitor {
    type Value = Quote;

    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        write!(formatter, "a map with keys 'first' and 'second'")
    }

    fn visit_seq<A>(self, mut seq: A) -> Result<Self::Value, A::Error>
    where
        A: SeqAccess<'de>,
    {
        let mut price = None;
        let mut amount = None;

        if let Some(val) = seq.next_element::<&str>()? {
            match val.parse::<f64>() {
                Ok(num) => price = Some(num),
                Err(_) => return Err(serde::de::Error::custom("Fail to convert price str to f64")),
            }
        }

        if let Some(val) = seq.next_element::<&str>()? {
            match val.parse::<f64>() {
                Ok(num) => amount = Some(num),
                Err(_) => {
                    return Err(serde::de::Error::custom(
                        "Fail to convert amount str to f64",
                    ))
                }
            }
        }

        if price.is_none() {
            return Err(serde::de::Error::custom("Missing price field"));
        }

        if amount.is_none() {
            return Err(serde::de::Error::custom("Missing amount field"));
        }

        Ok(Quote {
            price: price.unwrap(),
            amount: amount.unwrap(),
        })
    }
}


pub trait SharedT<Event>{
    type BinanceSnapshot;
    /// return last_update_id
    fn id(&self) -> i64;

    fn load_snapshot(&mut self, snapshot: &Self::BinanceSnapshot);

    /// Only used for "Event"
    fn add_event(&mut self, event: Event);
}

pub trait EventT{
    fn matches(&self, snap_shot_id: i64) -> bool;
    fn behind(&self, snap_shot_id: i64) -> bool;
    fn ahead(&self, snap_shot_id: i64) -> bool;
    fn equals(&self, snap_shot_id: i64) -> bool;
}

pub trait StreamEventT{
    type Event;
    fn event(&self) -> Self::Event;
}

pub trait SnapshotT{
    fn id(&self) -> i64;

    fn bids(&self) -> &Vec<Quote>;

    fn asks(&self) -> &Vec<Quote>;
}
