use crate::crypto::format::BookEventStream;
use crate::{Depth, Quote};
use ordered_float::OrderedFloat;
use serde::de::{SeqAccess, Visitor};
use serde::{Deserialize, Deserializer};
use std::collections::BTreeMap;
use std::fmt;
use std::fmt::Debug;
use std::time::{SystemTime, UNIX_EPOCH};

pub struct BookShared {
    instrument: String,
    last_update_id: i64,
    send_time: i64,
    receive_time: i64,
    asks: BTreeMap<OrderedFloat<f64>, f64>,
    bids: BTreeMap<OrderedFloat<f64>, f64>,
}

impl BookShared {
    pub fn new() -> Self {
        BookShared {
            instrument: String::new(),
            last_update_id: 0,
            send_time: 0,
            receive_time: 0,
            asks: BTreeMap::new(),
            bids: BTreeMap::new(),
        }
    }

    /// Only used for "LevelEvent"
    pub fn set_level_event(&mut self, level_event: BookEventStream) {
        self.asks.clear();
        self.bids.clear();

        let instrument = level_event.result.instrument_name;
        let data = level_event.result.data;
        let id = level_event.id;
        let mut send_time = 0;
        let data_len = data.len();

        for BookData {
            asks,
            bids,
            publish_time,
            ..
        } in data
        {
            for ask in asks {
                let ask_count = ask.order_numbers as usize;
                for _ in 0..ask_count {
                    self.asks.insert(OrderedFloat(ask.price), ask.amount);
                }
            }

            for bid in bids {
                let bid_count = bid.order_numbers as usize;
                for _ in 0..bid_count {
                    self.bids.insert(OrderedFloat(bid.price), bid.amount);
                }
            }
            send_time += publish_time;
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

        Depth {
            id,
            ts,
            lts,
            bids,
            asks,
        }
    }
}

#[derive(Deserialize, Debug, Clone)]
pub struct BookEvent {
    pub channel: String,

    pub subscription: String,

    /// Something like "BTC_USDT"
    pub instrument_name: String,

    pub data: Vec<BookData>,

    /// Usually constant value `20` or `50`
    pub depth: i64,
}

#[derive(Deserialize, Clone)]
pub struct BookData {
    /// Some timestamp server tells us
    #[serde(rename = "t")]
    pub publish_time: i64,

    #[serde(rename = "tt")]
    pub last_update_time: i64,

    #[serde(rename = "u")]
    pub update_sequence: i64,

    #[serde(rename = "cs")]
    pub other: i64,

    pub asks: Vec<Quotes>,

    pub bids: Vec<Quotes>,
}

#[derive(Debug, Copy, Clone)]
pub struct Quotes {
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

impl Debug for BookData {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Data")
            .field("publish_time", &self.publish_time)
            .field("last_update_time", &self.last_update_time)
            .field("update_sequence", &self.update_sequence)
            .field("asks", &self.asks.len())
            .field("bids", &self.bids.len())
            .field("other", &self.other)
            .finish()
    }
}
