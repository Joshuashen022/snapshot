use serde::{Deserialize, Deserializer, Serialize};
use std::collections::BTreeMap;
use std::fmt::{self, Debug};
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Deserialize, Serialize, Debug)]
pub struct HeartbeatRequest {
    pub id: i64,
    pub method: String,
    pub code: i64,
}

#[derive(Deserialize, Serialize)]
pub struct OrderRequest {
    pub id: i64,
    pub method: String,
    pub params: Params,
}

#[derive(Deserialize, Serialize)]
pub struct Params {
    pub channels: Vec<String>,
}

pub fn subscribe_message(channel: String) -> String {
    let _time = SystemTime::now().duration_since(UNIX_EPOCH).unwrap();
    let inner = OrderRequest {
        id: 1,
        method: String::from("subscribe"),
        params: Params {
            channels: vec![channel],
        },
    };
    serde_json::to_string(&inner).unwrap()
}
