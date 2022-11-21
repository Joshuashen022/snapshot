use crate::binance::connection::connect::{
    deserialize_event_with_stream, socket_stream, try_get_connection,
};
use crate::binance::format::binance_spot::{
    BinanceSnapshotSpot, EventSpot, LevelEventSpot, SharedSpot,
};
use crate::binance::format::{EventT, SharedT, SnapshotT, StreamEventT};
use crate::Depth;
use anyhow::Result;
use futures_util::future::Shared;
use futures_util::StreamExt;
use serde::de::DeserializeOwned;
use std::marker::PhantomData;
use std::marker::Send;
use std::sync::{Arc, Mutex, RwLock};
use std::fmt::Debug;
use tokio::sync::mpsc;
use tokio::sync::mpsc::UnboundedReceiver;
use tracing::{debug, error, info};
#[derive(Clone)]
pub struct BinanceOrderBook<Shared> {
    status: Arc<Mutex<bool>>,
    shared: Arc<RwLock<Shared>>,
}

impl<Event, Snapshot, Shared, StreamEvent, StreamLevelEvent>
    BinanceSnapshotT<Event, Snapshot, Shared, StreamEvent, StreamLevelEvent>
    for BinanceOrderBook<Shared>
where
    Event: DeserializeOwned + EventT + Send + Sync,
    Snapshot: SnapshotT + DeserializeOwned + Send + Sync,
    Shared: SharedT<Event, BinanceSnapshot = Snapshot, LevelEvent = StreamLevelEvent>
        + Send
        + Sync
        + 'static,
    StreamEvent: StreamEventT + DeserializeOwned + StreamEventT<Event = Event> + Send + Sync,
    StreamLevelEvent:
        StreamEventT + DeserializeOwned + StreamEventT<Event = Event> + Send + Sync + Debug,
{
    fn depth(
        &self,
        rest_address: String,
        depth_address: String,
    ) -> Result<UnboundedReceiver<Depth>> {
        let shared = self.shared.clone();
        let status = self.status.clone();
        let (sender, receiver) = mpsc::unbounded_channel();
        let sender = sender.clone();
        // Thread to maintain Order Book
        let _ = tokio::spawn(async move {
            info!("Start OrderBook thread");
            loop {
                let res = try_get_connection::<Event, Snapshot, Shared, StreamEvent>(
                    sender.clone(),
                    rest_address.clone(),
                    depth_address.clone(),
                    status.clone(),
                    shared.clone(),
                )
                .await;

                match res {
                    Ok(false) => error!("Try get connection failed retrying"),
                    Err(e) => error!("Error happen when try get connection {:?}", e),
                    _ => unreachable!(),
                }
            }
        });

        Ok(receiver)
    }

    fn level_depth(&self, level_address: String) -> Result<UnboundedReceiver<Depth>> {
        let shared = self.shared.clone();

        // This is not actually used
        let status = self.status.clone();
        let (sender, receiver) = mpsc::unbounded_channel();

        let _ = tokio::spawn(async move {
            info!("Start Level Buffer maintain thread");
            loop {
                if let Ok(mut guard) = status.lock() {
                    (*guard) = false;
                }
                let mut stream = match socket_stream(&level_address).await {
                    Ok(stream) => stream,
                    Err(e) => {
                        error!("Error calling {}, {:?}", level_address, e);
                        continue;
                    }
                };

                info!("Successfully connected to {}", level_address);
                if let Ok(mut guard) = status.lock() {
                    (*guard) = true;
                }

                info!("Level Overbook initialize success, now keep listening ");
                while let Ok(message) = stream.next().await.unwrap() {
                    let level_event = match deserialize_event_with_stream::<StreamLevelEvent>(
                        message.clone(),
                        &mut stream,
                    )
                    .await
                    {
                        Some(event) => event,
                        None => continue,
                    };
                    // todo: debug
                    // debug!("Level Event {}", level_event.last_update_id);

                    if let Ok(mut guard) = shared.write() {
                        (*guard).set_level_event(level_event);

                        let snapshot = (*guard).get_snapshot().depth();

                        if let Err(_) = sender.send(snapshot) {
                            error!("level_depth send Snapshot error");
                        };
                    } else {
                        error!("SharedSpot is busy");
                    }
                }
            }
        });

        Ok(receiver)
    }

    fn snapshot(&self) -> Option<Depth> {
        let mut current_status = false;

        if let Ok(status_guard) = self.status.lock() {
            current_status = (*status_guard).clone();
        } else {
            error!("BinanceSpotOrderBook lock is busy");
        }

        if current_status {
            Some(self.shared.write().unwrap().get_snapshot().depth())
        } else {
            None
        }
    }
}
pub trait BinanceSnapshotT<Event, Snapshot, Shared, StreamEvent, StreamLevelEvent>
where
    Event: DeserializeOwned + EventT,
    Snapshot: SnapshotT + DeserializeOwned,
    Shared: SharedT<Event, BinanceSnapshot = Snapshot>,
    StreamEvent: StreamEventT + DeserializeOwned + StreamEventT<Event = Event>,
{
    fn depth(
        &self,
        rest_address: String,
        depth_address: String,
    ) -> Result<UnboundedReceiver<Depth>>;

    fn level_depth(&self, level_address: String) -> Result<UnboundedReceiver<Depth>>;

    fn snapshot(&self) -> Option<Depth>;
}
