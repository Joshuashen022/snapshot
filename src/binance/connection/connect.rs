use tokio::net::TcpStream;
use tokio_tungstenite::{connect_async, tungstenite, MaybeTlsStream, WebSocketStream};
use tungstenite::{Message, WebSocket};
use anyhow::Result;
use crate::Depth;
use url::Url;
use serde::de::DeserializeOwned;
use std::sync::{Arc, Mutex, RwLock};
use crate::binance::format::{SharedT, EventT, SnapshotT, StreamEventT};
use std::collections::VecDeque;
use tracing::{debug, error, info, warn};
use futures_util::StreamExt;
use tokio::sync::mpsc::UnboundedSender;

const MAX_BUFFER_EVENTS: usize = 5;

pub type BinanceWebSocket = WebSocketStream<MaybeTlsStream<TcpStream>>;

pub async fn socket_stream(address: &str) -> Result<BinanceWebSocket, String>{
    let url = Url::parse(&address).expect("Bad URL");

    match connect_async(url).await{
        Ok((connection, _)) => Ok(connection),
        Err(e) => Err(format!("{:?}", e))
    }

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

fn add_event_to_orderbook<
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

pub async fn initialize<
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
    let snapshot: Snapshot =
        reqwest::get(&rest_address).await?.json().await?;

    info!("Successfully connected to {}", rest_address);

    let mut overbook_setup = false;
    let shared_clone = shared.clone();
    while let Some(event) = buffer_events.pop_front() {

        if let Ok(add_success) = add_event_to_orderbook::<Event, Snapshot, Shard>
            (event, shared_clone.clone(), &snapshot){
            if add_success{
                return Ok(true)
            }
        } else{
            return Ok(false)
        };
    }

    if overbook_setup {

        while let Some(event) = buffer_events.pop_front() {
            let mut orderbook = shared.write().unwrap();
            orderbook.add_event(event);
        }
    } else {
        info!(" Try to wait new events for out snapshot");

        while let Ok(message) = stream.next().await.unwrap() {
            let event = deserialize_event::<StreamEvent>(message).unwrap();
            let event = event.event();

            if let Ok(add_success) = add_event_to_orderbook::<Event, Snapshot, Shard>
                (event, shared_clone.clone(), &snapshot){
                if add_success{
                    return Ok(true)
                }
            } else{
                return Ok(false)
            };
        }
    }

    Ok(false)
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
) -> Result<bool>{
        if let Ok(mut guard) = status.lock() {
            (*guard) = false;
        }
        let mut stream = match socket_stream(&depth_address).await {
            Ok(stream) => stream,
            Err(e) => {
                error!("Error calling {}, {:?}", depth_address, e);
                return Ok(false)
            }
        };

        info!("Successfully connected to {}", depth_address);
        match initialize::<Event, Snapshot, Shard, StreamEvent>
            (&mut stream, rest_address.clone(), shared.clone()).await
        {
            Ok(overbook_setup) => {
                if overbook_setup {
                    if let Ok(mut guard) = status.lock(){
                        (*guard) = true;
                    };
                } else {
                    warn!("All event is not usable, need a new snapshot");
                    return Ok(false)
                }
            },
            Err(e) => {
                error!("{:?}",e);
                return Ok(false)
            }
        };

        info!(" Overbook initialize success, now keep listening ");

        while let Ok(message) = stream.next().await.unwrap() {
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