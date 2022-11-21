use crate::binance::connection::connect::{socket_stream, try_get_connection};
use crate::binance::format::binance_spot::{
    BinanceSnapshotSpot, EventSpot, LevelEventSpot, SharedSpot,
};
use crate::binance::format::SharedT;
use crate::Depth;

use anyhow::anyhow;
use anyhow::{Error, Result};
use futures_util::StreamExt;
use std::sync::{Arc, Mutex, RwLock};
use tokio::sync::mpsc::{self, UnboundedReceiver};
use tracing::{debug, error, info, warn};

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
        let (sender, receiver) = mpsc::unbounded_channel();
        let sender = sender.clone();
        // Thread to maintain Order Book
        let _ = tokio::spawn(async move {
            info!("Start OrderBook thread");
            loop {
                let res =
                    try_get_connection::<EventSpot, BinanceSnapshotSpot, SharedSpot, EventSpot>(
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
                    _ => unreachable!()
                }
            }
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
                while let Ok(msg) = stream.next().await.unwrap() {
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

    #[allow(dead_code)]
    fn update_status(&mut self, value: bool) -> Result<()> {
        if let Ok(mut guard) = self.status.lock() {
            (*guard) = value;
        }
        Ok(())
    }
}
