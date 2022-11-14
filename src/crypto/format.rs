use ordered_float::OrderedFloat;
use std::collections::BTreeMap;
use std::fmt;
use std::time::{SystemTime, UNIX_EPOCH};
use serde::{Deserialize, Deserializer};
use serde::de::{SeqAccess, Visitor};
use tracing::{debug, Instrument};
use crate::binance::format::binance_spot::{LevelEventSpot, SharedSpot};
use crate::{BinanceOrderBookSnapshot, Depth, Quote};

pub struct Shared {
    instrument: String,
    last_update_id: i64,
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
            send_time: 0,
            receive_time: 0,
            asks: BTreeMap::new(),
            bids: BTreeMap::new(),
        }
    }

    /// Only used for "LevelEvent"
    pub fn set_level_event(&mut self, level_event: LevelEventStream) {
        self.asks.clear();
        self.bids.clear();

        let instrument = level_event.result.instrument_name;
        let data = level_event.result.data;
        let id = level_event.id;
        let mut send_time = 0;
        let data_len = data.len();
        for Data{t, asks, bids} in data{

            for ask in asks {
                self.asks.insert(OrderedFloat(ask.price), ask.amount);
            }

            for bid in bids {
                self.bids.insert(OrderedFloat(bid.price), bid.amount);
            }
            send_time += t;

        }
        send_time /= data_len as i64;

        let time = SystemTime::now().duration_since(UNIX_EPOCH).unwrap();
        self.instrument = instrument;
        self.last_update_id = id;
        self.send_time = send_time;
        self.receive_time = time.as_millis() as i64;
    }

    pub fn get_snapshot(&self) -> Depth {
        let id = self.last_update_id;
        let ts = self.send_time;
        let lts = self.receive_time;

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

        Depth{
            id, ts, lts, bids, asks
        }
    }
}

#[derive(Deserialize, Debug)]
pub struct LevelEventStream{
    /// Usually constant value `-1`
    pub id: i64,

    /// Something like: "public/get-block"
    pub method: String,

    /// Usually constant value `0`
    pub code: i64,

    pub result: Event,
}

impl LevelEventStream {
    pub fn debug(&self){
        debug!(
            "receive level_event depth {}, data {}",
            self.result.depth,
            self.result.data.len(),
        );

        for data in self.result.data.clone(){
            debug!("bids {}, asks {}, time {}",
                data.bids.len(),
                data.asks.len(),
                data.t,
            )
        };
    }
}

#[derive(Deserialize, Debug, Clone)]
pub struct Event{
    /// Usually constant value `20` or `50`
    pub depth: i64,

    pub data: Vec<Data>,

    /// Something like "BTC_USDT"
    pub instrument_name: String
}

#[derive(Deserialize, Debug, Clone)]
pub struct Data{
    pub bids: Vec<Quotes>,

    pub asks: Vec<Quotes>,

    /// Some timestamp server tells us
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
