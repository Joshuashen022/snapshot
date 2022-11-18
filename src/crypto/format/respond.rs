use serde::{Deserialize, Serialize};
use std::fmt::Debug;
use tokio_tungstenite::tungstenite::protocol::Message;
#[derive(Deserialize, Serialize)]
pub struct HeartbeatRespond {
    pub id: i64,
    pub method: String,
}

pub fn heartbeat_respond(id: i64) -> Message {
    let inner = HeartbeatRespond {
        id,
        method: String::from("public/respond-heartbeat"),
    };
    let inner = serde_json::to_string(&inner).unwrap();
    Message::from(inner)
}

#[derive(Deserialize, Serialize, PartialEq, Eq, Debug)]
pub struct GeneralRespond {
    pub id: i64,
    pub code: i64,
    pub method: String,
}

#[derive(Deserialize, Serialize, PartialEq, Eq, Debug)]
pub struct OrderRespond {
    pub id: i64,
    pub code: i64,
    pub method: String,
    pub channel: String,
}
