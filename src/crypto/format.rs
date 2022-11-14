use ordered_float::OrderedFloat;
use std::collections::BTreeMap;
use std::fmt;
use serde::{Deserialize, Deserializer};
use serde::de::{SeqAccess, Visitor};
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
    pub id: i64,
    pub method: String,
    pub code: i64,
    pub result: Event,
}

#[derive(Deserialize, Debug, Clone)]
pub struct Event{
    pub depth: i64,
    pub data: Vec<Data>,
    pub instrument_name: String
}

#[derive(Deserialize, Debug, Clone)]
pub struct Data{
    pub bids: Vec<Quotes>,
    pub asks: Vec<Quotes>,
    pub t: i64,
}

#[derive(Debug, Copy, Clone)]
pub struct Quotes{
    price: f64,
    amount: f64,
    order_numbers: i64,
}

impl<'de> Deserialize<'de> for Quotes {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
        where
            D: Deserializer<'de>,
    {
        deserializer.deserialize_tuple(3, QuotesVisitor)
    }
}

struct QuotesVisitor;

impl<'de> Visitor<'de> for QuotesVisitor {
    type Value = Quotes;

    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        write!(formatter, "a map with keys 'first' and 'second'")
    }

    fn visit_seq<A>(self, mut seq: A) -> Result<Self::Value, A::Error>
        where
            A: SeqAccess<'de>,
    {
        let mut price = None;
        let mut amount = None;
        let mut order_numbers = None;

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

        if let Some(val) = seq.next_element::<&str>()? {
            match val.parse::<i64>() {
                Ok(num) => order_numbers = Some(num),
                Err(_) => {
                    return Err(serde::de::Error::custom(
                        "Fail to convert order_numbers str to u64",
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

        if order_numbers.is_none() {
            return Err(serde::de::Error::custom("Missing order_numbers field"));
        }

        Ok(Quotes {
            price: price.unwrap(),
            amount: amount.unwrap(),
            order_numbers: order_numbers.unwrap(),
        })
    }
}
