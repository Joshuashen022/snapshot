use serde::Deserialize;

#[derive(Deserialize, Debug, Clone)]
pub struct TradeEvent{
    pub channel: String,

    pub subscription: String,

    /// Something like "BTC_USDT"
    pub instrument_name: String,

    pub data: Vec<TradeData>,
}

#[derive(Deserialize, Debug, Clone)]
pub struct TradeData{
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

