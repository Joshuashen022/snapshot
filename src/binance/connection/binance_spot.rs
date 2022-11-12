use crate::binance::format::{SharedT, EventT, SnapshotT, StreamEventT};

use std::collections::VecDeque;
use std::sync::{Arc, Mutex, RwLock};
use std::time::{SystemTime, UNIX_EPOCH};
// use crate::binance::connection::BinanceOrderBookSnapshot;
use tokio::sync::mpsc::{self, UnboundedReceiver};
use tokio_tungstenite::{connect_async, MaybeTlsStream, tungstenite, WebSocketStream};
use futures_util::StreamExt;
use tungstenite::{Message, WebSocket};
use anyhow::anyhow;
use anyhow::{Error, Result};
use tokio::net::TcpStream;
use url::Url;


use tracing::{debug, error, info, warn};
use crate::binance::format::binance_spot::{
    BinanceSnapshotSpot, EventSpot, LevelEventSpot, SharedSpot,
};
use crate::{Depth, OrderBookSnapshot};
use crate::binance::connection::connect::{
    socket_stream, BinanceWebSocket, initialize
};

const MAX_BUFFER_EVENTS: usize = 5;

#[derive(Clone)]
pub struct BinanceOrderBookSpot {
    status: Arc<Mutex<bool>>,
    shared: Arc<RwLock<SharedSpot>>,
}

impl BinanceOrderBookSpot {
    pub fn new() -> Self {
        BinanceOrderBookSpot {
            status: Arc::new(Mutex::new(false)),
            shared: Arc::new(RwLock::new(SharedSpot::new())),
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
        let self_clone = self.clone();
        let (sender, receiver) = mpsc::unbounded_channel();
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
                    match initialize::<EventSpot, BinanceSnapshotSpot, SharedSpot, EventSpot>
                        (&mut stream, rest_address.clone(), shared.clone()).await
                    {
                        Ok(overbook_setup) => {
                            if overbook_setup {
                                if let Ok(mut guard) = status.lock(){
                                    (*guard) = true;
                                };
                            } else {
                                warn!("All event is not usable, need a new snapshot");
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
                        if event.ahead(orderbook.id()) {
                            warn!("All event is not usable, need a new snapshot");
                            break;
                        } else if event.equals(orderbook.id()) {
                            orderbook.add_event(event);

                            let snapshot = orderbook.get_snapshot();

                            if let Err(_) = sender.send(snapshot.depth()) {
                                warn!("depth send Snapshot error");
                            };
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

    // TODO:: deal with error at outer_space
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

                    let level_event: LevelEventSpot = match serde_json::from_str(&text) {
                        Ok(e) => e,
                        Err(e) => {
                            warn!("Error {}, {:?}", e, msg);
                            continue;
                        }
                    };

                    debug!("Level Event {}", level_event.last_update_id);

                    let time = SystemTime::now().duration_since(UNIX_EPOCH).unwrap();
                    if let Ok(mut guard) = shared.write() {
                        (*guard).set_level_event(level_event, time.as_millis() as i64);

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

    fn update_status(&mut self, value: bool) -> Result<()>{
        if let Ok(mut guard) = self.status.lock() {
            (*guard) = value;
        }
        Ok(())
    }
}

fn deserialize_message(message: Message) -> Option<EventSpot> {
    if !message.is_text() {
        return None;
    }

    let text = match message.into_text() {
        Ok(e) => e,
        Err(_) => return None,
    };

    let event: EventSpot = match serde_json::from_str(&text) {
        Ok(e) => e,
        Err(_) => return None,
    };

    Some(event)
}
