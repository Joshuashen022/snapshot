use std::collections::BTreeMap;
use std::fmt;
use std::time::{SystemTime, UNIX_EPOCH};
use ordered_float::OrderedFloat;
use serde::{Deserialize, Deserializer};
use serde::de::{SeqAccess, Visitor};
pub use crate::crypto::format::stream::BookEventStream;
use crate::{Depth, Quote};

pub mod stream;
pub mod respond;
pub mod request;
pub mod trade;
pub mod book;

pub use request::subscribe_message;
pub use respond::GeneralRespond;
pub use respond::OrderRespond;
pub use respond::heartbeat_respond;
pub use request::HeartbeatRequest;
pub use book::BookEvent;
use crate::crypto::format::book::BookData;


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

        /// Some timestamp server tells us

        for BookData { asks, bids, publish_time, .. } in data {
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
