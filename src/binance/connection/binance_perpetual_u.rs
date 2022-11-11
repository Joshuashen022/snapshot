use crate::binance::format::binance_perpetual_u::{
    BinanceSnapshotPerpetualU, EventPerpetualU, SharedPerpetualU, StreamEventPerpetualU,
    StreamLevelEventPerpetualU,
};
use crate::Depth;
use crate::binance::connection::connect::{socket_stream, BinanceWebSocket};

use anyhow::anyhow;
use anyhow::{Error, Result};
use futures_util::StreamExt;
use std::collections::vec_deque::VecDeque;
use tokio::sync::mpsc::{self, UnboundedReceiver};
use tokio_tungstenite::connect_async;
use tracing::{debug, error, info, warn};
use url::Url;
// use tokio::select;
use std::sync::{Arc, Mutex, RwLock};
// use std::time::{Instant, SystemTime, UNIX_EPOCH};
// use futures_util::future::err;
use tokio_tungstenite::tungstenite::Message;
// use tokio::spawn;

// const DEPTH_URL: &str = "wss://fstream.binance.com/stream?streams=btcusdt@depth@100ms";
// const LEVEL_DEPTH_URL: &str = "wss://fstream.binance.com/stream?streams=btcusdt@depth20@100ms";
// const REST: &str = "https://fapi.binance.com/fapi/v1/depth?symbol=BTCUSDT&limit=1000";
// const MAX_BUFFER: usize = 30;
const MAX_BUFFER_EVENTS: usize = 5;

#[derive(Clone)]
pub struct BinanceSpotOrderBookPerpetualU {
    status: Arc<Mutex<bool>>,
    pub(crate) shared: Arc<RwLock<SharedPerpetualU>>,
}

impl BinanceSpotOrderBookPerpetualU {
    pub fn new() -> Self {
        BinanceSpotOrderBookPerpetualU {
            status: Arc::new(Mutex::new(false)),
            shared: Arc::new(RwLock::new(SharedPerpetualU::new())),
        }
    }

    /// acquire a order book with "depth method"
    pub fn depth(
        &self,
        rest_address: String,
        depth_address: String,
    ) -> Result<UnboundedReceiver<Depth>> {
        let shared = self.shared.clone();
        let status = self.status.clone();
        let (sender, receiver) = mpsc::unbounded_channel();
        let self_clone = self.clone();
        // Thread to maintain Order Book
        let _ = tokio::spawn(async move {
            let mut default_exit = 0;
            info!("Start OrderBook thread");
            loop {
                let res: Result<()> = {
                    if let Ok(mut guard) = status.lock() {
                        (*guard) = false;
                    }

                    let mut stream = match socket_stream(&depth_address).await {
                        Ok(stream) => stream,
                        Err(e) => {
                            error!("Error calling {}, {:?}", depth_address, e);
                            default_exit += 1;
                            continue
                        }
                    };
                    info!("Successfully connected to {}", depth_address);
                    match self_clone.clone().initialize(&mut stream, rest_address.clone()).await{
                        Ok(overbook_setup) => {
                            if overbook_setup {
                                if let Ok(mut guard) = status.lock(){
                                    (*guard) = true;
                                };
                            } else {
                                continue
                            }
                        },
                        Err(e) => {
                            error!("{:?}",e);
                            continue
                        }
                    };

                    info!(" Overbook initialize success, now keep listening ");

                    while let Ok(message) = stream.next().await.unwrap() {
                        let event = deserialize_message(message.clone());
                        if event.is_none() {
                            warn!("Message decode error {:?}", message);
                            continue;
                        }
                        let event = event.unwrap();

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

                    Ok(())
                };

                match res {
                    Ok(_) => (),
                    Err(e) => error!("Error happen when running code: {:?}", e),
                }

                if default_exit > 20 {
                    error!("Using default break");
                    break;
                }

                default_exit += 1;
            }
            error!("OrderBook thread stopped");
            Ok::<(), Error>(())
        });

        Ok(receiver)
    }

    pub fn level_depth(&self, level_address: String) -> Result<UnboundedReceiver<Depth>> {
        let shared = self.shared.clone();

        // This is not actually used
        let status = self.status.clone();

        let (sender, receiver) = mpsc::unbounded_channel();

        let _ = tokio::spawn(async move {
            info!("Start Level Buffer maintain thread");
            loop {
                let url = Url::parse(&level_address).expect("Bad URL");

                let res = connect_async(url).await;
                let mut stream = match res {
                    Ok((stream, _)) => stream,
                    Err(e) => {
                        error!("Error {:?}, reconnecting {}", e, level_address);
                        continue;
                    }
                };

                info!("Successfully connected to {}", level_address);

                if let Ok(mut guard) = status.lock() {
                    (*guard) = true;
                }

                info!("Level Overbook initialize success, now keep listening ");

                while let Ok(msg) = stream.next().await.unwrap() {
                    //
                    if !msg.is_text() {
                        warn!("msg is empty");
                        continue;
                    }

                    let text = match msg.clone().into_text() {
                        Ok(e) => e,
                        Err(e) => {
                            warn!("msg.into_text {:?}", e);
                            continue;
                        }
                    };

                    let level_event: StreamLevelEventPerpetualU = match serde_json::from_str(&text)
                    {
                        Ok(e) => e,
                        Err(e) => {
                            warn!("Error {},{:?}", e, msg);
                            continue;
                        }
                    };
                    let level_event = level_event.data;

                    debug!(
                        "receive level_event {}-{}({}) ts: {}",
                        level_event.first_update_id,
                        level_event.last_update_id,
                        level_event.last_message_last_update_id,
                        level_event.event_time,
                    );

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

    /// Get the snapshot of the current Order Book
    pub fn snapshot(&self) -> Option<Depth> {
        let mut current_status = false;

        if let Ok(status_guard) = self.status.lock() {
            current_status = (*status_guard).clone();
        } else {
            error!("BinanceSpotOrderBookPerpetualU lock is busy");
        }

        if current_status {
            Some(self.shared.write().unwrap().get_snapshot().depth())
        } else {
            debug!("data is not ready");
            None
        }
    }

    pub(crate) fn set_symbol(&mut self, symbol: String) -> Result<()> {
        {
            match self.shared.clone().write() {
                Ok(mut shared) => {
                    (*shared).symbol = symbol;
                    Ok(())
                }
                Err(e) => Err(anyhow!("{:?}", e)),
            }
        }
    }

    async fn initialize(
        &self,
        stream: &mut BinanceWebSocket,
        rest_address: String,
    ) -> Result<bool> {

        let mut buffer_events = VecDeque::new();

        while let Ok(message) = stream.next().await.unwrap() {
            let event = deserialize_message(message);
            if event.is_none() {
                continue;
            }
            let event = event.unwrap();

            buffer_events.push_back(event);

            if buffer_events.len() == MAX_BUFFER_EVENTS {
                break;
            }
        }

        // Wait for a while to collect event into buffer
        let snapshot: BinanceSnapshotPerpetualU =
            reqwest::get(&rest_address).await?.json().await?;

        info!("Successfully connected to {}", rest_address);

        let snap_shot_id = snapshot.last_update_id;

        let mut overbook_setup = false;
        while let Some(event) = buffer_events.pop_front() {

            if event.behind(snap_shot_id) {
                continue
            }

            if event.matches(snap_shot_id) {
                let mut orderbook = self.shared.write().unwrap();
                orderbook.load_snapshot(&snapshot);
                orderbook.add_event(event);
                overbook_setup = true;
                return Ok(true)
            }

            if event.ahead(snap_shot_id) {
                warn!("All event is not usable, need a new snap shot ");
                return Ok(false)
            }
        }

        if overbook_setup {
            debug!("Emptying events in buffer");

            while let Some(event) = buffer_events.pop_front() {
                let mut orderbook = self.shared.write().unwrap();
                orderbook.add_event(event);
            }
        } else {
            info!(" Try to wait new events for out snapshot");

            while let Ok(message) = stream.next().await.unwrap() {
                let event = deserialize_message(message);
                if event.is_none() {
                    continue;
                }

                let event = event.unwrap();
                if event.behind(snap_shot_id) {
                    continue;
                }

                let mut orderbook = self.shared.write().unwrap();
                if event.matches(snap_shot_id) {
                    orderbook.load_snapshot(&snapshot);
                    orderbook.add_event(event);
                    overbook_setup = true;
                    return Ok(true)
                }

                if event.ahead(snap_shot_id) {
                    break;
                }
            }
        }

        Ok(false)
    }
}

fn deserialize_message(message: Message) -> Option<EventPerpetualU> {
    if !message.is_text() {
        return None;
    }

    let text = match message.into_text() {
        Ok(e) => e,
        Err(_) => return None,
    };

    let s_event: StreamEventPerpetualU = match serde_json::from_str(&text) {
        Ok(e) => e,
        Err(_) => return None,
    };

    let event = s_event.data;

    Some(event)
}
