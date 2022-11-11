use tokio::net::TcpStream;
use tokio_tungstenite::{connect_async, tungstenite, MaybeTlsStream, WebSocketStream};
use tungstenite::{Message, WebSocket};
use anyhow::Result;
use url::Url;
use serde::de::DeserializeOwned;
use std::sync::{Arc, RwLock};
use crate::binance::format::{SharedT, EventT, SnapshotT};

pub type BinanceWebSocket = WebSocketStream<MaybeTlsStream<TcpStream>>;

pub async fn socket_stream(address: &str) -> Result<BinanceWebSocket, String>{
    let url = Url::parse(&address).expect("Bad URL");

    match connect_async(url).await{
        Ok((connection, _)) => Ok(connection),
        Err(e) => Err(format!("{:?}", e))
    }

}

pub fn deserialize_event<Event: DeserializeOwned>(message: Message) -> Option<Event> {
    if !message.is_text() {
        return None;
    }

    let text = match message.into_text() {
        Ok(e) => e,
        Err(_) => return None,
    };

    let event: Event = match serde_json::from_str(&text) {
        Ok(e) => e,
        Err(_) => return None,
    };

    Some(event)
}

pub fn add_event_to_orderbook<
    Event: EventT,
    Snapshot: SnapshotT,
    Shard: SharedT<Event, BinanceSnapshot = Snapshot>,
>(
    event: Event,
    shared: Arc<RwLock<Shard>>,
    snapshot: &Snapshot
) -> Result<bool>{
    let snap_shot_id = snapshot.id();
    if event.behind(snap_shot_id) {
        return Ok(false)
    }

    if event.matches(snap_shot_id) {
        let mut orderbook = shared.write().unwrap();
        orderbook.load_snapshot(&snapshot);
        orderbook.add_event(event);
        return Ok(true)
    }

    if event.ahead(snap_shot_id) {
        return Ok(false)
    }

    Ok(false)
}