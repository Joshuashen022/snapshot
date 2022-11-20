use crate::crypto::format::depth::DepthEvent;
use crate::crypto::format::ticker::TickerEvent;
use serde::Deserialize;

#[derive(Deserialize, Debug)]
pub struct DepthEventStream {
    /// Usually constant value `-1`
    pub id: i64,

    /// Something like: "public/get-block"
    pub method: String,

    /// Usually constant value `0`
    pub code: i64,

    pub result: DepthEvent,
}

#[derive(Deserialize, Debug)]
pub struct TickerEventStream {
    /// Usually constant value `-1`
    pub id: i64,

    /// Something like: "public/get-block"
    pub method: String,

    /// Usually constant value `0`
    pub code: i64,

    pub result: TickerEvent,
}
