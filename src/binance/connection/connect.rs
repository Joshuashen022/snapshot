use crate::binance::format::{EventT, SharedT, SnapshotT, StreamEventT};
use crate::Depth;

use anyhow::{anyhow, Result};
use futures_util::{SinkExt, StreamExt};
use serde::de::DeserializeOwned;
use std::collections::VecDeque;
use std::sync::{Arc, Mutex, RwLock};
use std::time::Duration;
use tokio::net::TcpStream;
use tokio::sync::mpsc::UnboundedSender;
use tokio::time::sleep;
use tokio_tungstenite::{connect_async, tungstenite, MaybeTlsStream, WebSocketStream};
use tracing::{debug, error, info, warn};
use tungstenite::Message;
use url::Url;

const MAX_BUFFER_EVENTS: usize = 5;

pub type BinanceWebSocket = WebSocketStream<MaybeTlsStream<TcpStream>>;

pub async fn socket_stream(address: &str) -> Result<BinanceWebSocket, String> {
    let url = Url::parse(&address).expect("Bad URL");

    match connect_async(url).await {
        Ok((connection, _)) => Ok(connection),
        Err(e) => Err(format!("{:?}", e)),
    }
}

pub async fn try_get_connection<
    Event: DeserializeOwned + EventT,
    Snapshot: SnapshotT + DeserializeOwned,
    Shard: SharedT<Event, BinanceSnapshot = Snapshot>,
    StreamEvent: StreamEventT + DeserializeOwned + StreamEventT<Event = Event>,
>(
    sender: UnboundedSender<Depth>,
    rest_address: String,
    depth_address: String,
    status: Arc<Mutex<bool>>,
    shared: Arc<RwLock<Shard>>,
) -> Result<bool> {
    if let Ok(mut guard) = status.lock() {
        (*guard) = false;
    }
    let mut stream = match socket_stream(&depth_address).await {
        Ok(stream) => stream,
        Err(e) => {
            error!("Error calling {}, {:?}", depth_address, e);
            sleep(Duration::from_millis(100)).await;
            return Ok(false);
        }
    };

    info!("Successfully connected to {}", depth_address);
    match initialize::<Event, Snapshot, Shard, StreamEvent>(
        &mut stream,
        rest_address.clone(),
        shared.clone(),
    )
    .await
    {
        Ok(overbook_setup) => {
            if overbook_setup {
                if let Ok(mut guard) = status.lock() {
                    (*guard) = true;
                };
            } else {
                warn!("All event is not usable, need a new snapshot");
                return Ok(false);
            }
        }
        Err(e) => {
            error!("{:?}", e);
            return Ok(false);
        }
    };

    info!(" Overbook initialize success, now keep listening ");

    while let Ok(message) = stream.next().await.unwrap() {
        if message.is_ping() {
            debug!("Receiving ping message");
            let inner = message.clone().into_data();
            match stream.send(Message::Pong(inner.clone())).await {
                Ok(_) => continue,
                Err(e) => {
                    warn!("Send pong error {:?}", e);
                    let _ = stream.send(Message::Pong(inner.clone())).await;
                }
            };
        }

        let event = deserialize_event::<StreamEvent>(message.clone());
        if event.is_none() {
            warn!("Message decode error {:?}", message);
            continue;
        }
        let event = event.unwrap().event();

        let mut orderbook = shared.write().unwrap();
        if event.equals(orderbook.id()) {
            orderbook.add_event(event);

            let snapshot = orderbook.get_snapshot();

            if let Err(_) = sender.send(snapshot.depth()) {
                error!("depth send Snapshot error");
            };
        } else {
            warn!("All event is not usable, need a new snapshot");
            break;
        }
    }
    Ok(false)
}

fn deserialize_event<StreamEvent: DeserializeOwned>(message: Message) -> Option<StreamEvent> {
    if !message.is_text() {
        return None;
    }

    let text = match message.into_text() {
        Ok(e) => e,
        Err(_) => return None,
    };

    let event: StreamEvent = match serde_json::from_str(&text) {
        Ok(e) => e,
        Err(_) => return None,
    };

    Some(event)
}

pub async fn deserialize_event_with_stream<StreamEvent: DeserializeOwned>(
    message: Message,
    stream: &mut BinanceWebSocket,
) -> Option<StreamEvent> {
    if message.is_ping() {
        debug!("Receiving ping message");
        let inner = message.clone().into_data();
        match stream.send(Message::Pong(inner.clone())).await {
            Ok(_) => return None,
            Err(e) => {
                warn!("Send pong error {:?}", e);
                let _ = stream.send(Message::Pong(inner.clone())).await;
            }
        };
    }

    if !message.is_text() {
        warn!("message is empty");
        return None;
    }

    let text = match message.clone().into_text() {
        Ok(e) => e,
        Err(e) => {
            warn!("message into text error {:?}", e);
            return None;
        }
    };

    let event: StreamEvent = match serde_json::from_str(&text) {
        Ok(e) => e,
        Err(e) => {
            warn!("Error {}, {:?}", e, message);
            return None;
        }
    };

    Some(event)
}

fn add_event_to_orderbook<
    Event: EventT,
    Snapshot: SnapshotT,
    Shard: SharedT<Event, BinanceSnapshot = Snapshot>,
>(
    event: Event,
    shared: Arc<RwLock<Shard>>,
    snapshot: &Snapshot,
) -> Result<bool> {
    let snap_shot_id = snapshot.id();
    if event.behind(snap_shot_id) {
        return Ok(false);
    }

    if event.matches(snap_shot_id) {
        let mut orderbook = shared.write().unwrap();
        orderbook.load_snapshot(&snapshot);
        orderbook.add_event(event);
        return Ok(true);
    }

    if event.ahead(snap_shot_id) {
        return Err(anyhow!(" Event is a head of snapshot"));
    }

    Ok(false)
}

async fn initialize<
    Event: DeserializeOwned + EventT,
    Snapshot: SnapshotT + DeserializeOwned,
    Shard: SharedT<Event, BinanceSnapshot = Snapshot>,
    StreamEvent: StreamEventT + DeserializeOwned + StreamEventT<Event = Event>,
>(
    stream: &mut BinanceWebSocket,
    rest_address: String,
    shared: Arc<RwLock<Shard>>,
) -> Result<bool> {
    let mut buffer_events = VecDeque::new();

    while let Ok(message) = stream.next().await.unwrap() {
        let event = deserialize_event::<StreamEvent>(message).unwrap();
        let event = event.event();
        buffer_events.push_back(event);

        if buffer_events.len() == MAX_BUFFER_EVENTS {
            break;
        }
    }

    // Wait for a while to collect event into buffer
    let snapshot: Snapshot = reqwest::get(&rest_address).await?.json().await?;

    info!("Successfully connected to {}", rest_address);

    let mut overbook_setup = false;
    let shared_clone = shared.clone();
    while let Some(event) = buffer_events.pop_front() {
        if let Ok(add_success) =
            add_event_to_orderbook::<Event, Snapshot, Shard>(event, shared_clone.clone(), &snapshot)
        {
            if add_success {
                overbook_setup = true;
            } // else: event behind snap_shot wait for next message
        } else {
            // Add event to order book error, need a new https snapshot
            return Ok(false);
        };
    }

    if overbook_setup {
        while let Some(event) = buffer_events.pop_front() {
            let mut orderbook = shared.write().unwrap();
            orderbook.add_event(event);
        }
        // default exit return Ok(true)
    } else {
        info!(" Try to wait new events for out snapshot");

        while let Ok(message) = stream.next().await.unwrap() {
            let event = deserialize_event::<StreamEvent>(message).unwrap();
            let event = event.event();

            if let Ok(add_success) = add_event_to_orderbook::<Event, Snapshot, Shard>(
                event,
                shared_clone.clone(),
                &snapshot,
            ) {
                if add_success {
                    // default exit return Ok(true)
                    break;
                } // else: event behind snap_shot wait for next message
            } else {
                warn!("All event is not usable, need a new snapshot");
                return Ok(false);
            };
        }
    }

    return Ok(true);
}
