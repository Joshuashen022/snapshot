use std::fmt;
use std::fmt::Debug;
use serde::Deserialize;
use crate::crypto::format::Quotes;

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
