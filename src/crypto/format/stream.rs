use crate::crypto::format::book::BookEvent;
use crate::crypto::format::trade::TradeEvent;
use serde::Deserialize;

#[derive(Deserialize, Debug)]
pub struct BookEventStream {
    /// Usually constant value `-1`
    pub id: i64,

    /// Something like: "public/get-block"
    pub method: String,

    /// Usually constant value `0`
    pub code: i64,

    pub result: BookEvent,
}

#[derive(Deserialize, Debug)]
pub struct TradeEventStream {
    /// Usually constant value `-1`
    pub id: i64,

    /// Something like: "public/get-block"
    pub method: String,

    /// Usually constant value `0`
    pub code: i64,

    pub result: TradeEvent,
}
