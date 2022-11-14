use crate::crypto::format::Shared;
use crate::{BinanceConnectionType, Depth};
use anyhow::Result;
use std::sync::{Arc, Mutex, RwLock};
use tokio::sync::mpsc;
use tokio::sync::mpsc::UnboundedReceiver;
use tokio_tungstenite::connect_async;
use tracing::{debug, error, info, warn};
use url::Url;
use crate::crypto::format::LevelEventStream;
use futures_util::StreamExt;

#[derive(Clone)]
pub struct CryptoOrderBookSpot {
    status: Arc<Mutex<bool>>,
    shared: Arc<RwLock<Shared>>,
}

impl CryptoOrderBookSpot {
    pub fn new() -> Self {
        CryptoOrderBookSpot {
            status: Arc::new(Mutex::new(false)),
            shared: Arc::new(RwLock::new(Shared::new())),
        }
    }

    pub fn level_depth(&self, level_address: String) -> Result<UnboundedReceiver<Depth>> {
        let shared = self.shared.clone();

        let status = self.status.clone();

        let (sender, receiver) = mpsc::unbounded_channel();

        let _ = tokio::spawn(async move {
            info!("Start Level Buffer maintain thread");
            loop {
                if let Ok(mut guard) = status.lock() {
                    (*guard) = false;
                }

                let url = Url::parse(&address).expect("Bad URL");

                let socket_stream = match connect_async(url).await {
                    Ok((connection, _)) => Ok(connection),
                    Err(e) => Err(format!("{:?}", e)),
                };

                let mut stream = match socket_stream{
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

                    let level_event: LevelEventStream = match serde_json::from_str(&text)
                    {
                        Ok(e) => e,
                        Err(e) => {
                            warn!("Error {},{:?}", e, msg);
                            continue;
                        }
                    };
                    let level_event = level_event.result;

                    debug!(
                        "receive level_event depth {}, ask {} bis {}",
                        level_event.depth,
                        level_event.data.asks.len(),
                        level_event.data.bids.len(),

                    );

                    // if let Ok(mut guard) = shared.write() {
                    //     (*guard).set_level_event(level_event);
                    //
                    //     let snapshot = (*guard).get_snapshot().depth();
                    //     if let Err(_) = sender.send(snapshot) {
                    //         error!("level_depth send Snapshot error");
                    //     };
                    // } else {
                    //     error!("SharedSpot is busy");
                    // }
                }
            }
        });

        Ok(receiver)
    }
}
