use crate::crypto::format::LevelEventStream;
use crate::crypto::format::Shared;
use crate::Depth;
use anyhow::{Error, Result};
use std::sync::{Arc, Mutex, RwLock};
use futures_util::StreamExt;
use tokio::sync::mpsc;
use tokio::sync::mpsc::UnboundedReceiver;
use tokio_tungstenite::connect_async;
use tracing::{error, info, warn};
use url::Url;

#[derive(Clone)]
pub struct CryptoOrderBookSpot {
    /// Currently not using
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
    #[allow(unreachable_code)]
    pub fn level_depth(&self, level_address: String) -> Result<UnboundedReceiver<Depth>> {
        let shared = self.shared.clone();

        let (sender, receiver) = mpsc::unbounded_channel();

        let _ = tokio::spawn(async move {
            info!("Start Level Buffer maintain thread");
            loop {
                let result: Result<()> = {

                    let url = Url::parse(&level_address).expect("Bad URL");

                    let mut stream = match connect_async(url).await {
                        Ok((connection, _)) => connection,
                        Err(e) => {
                            warn!("connection error {:?}", e);
                            continue
                        }
                    };

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

                        let level_event: LevelEventStream = match serde_json::from_str(&text) {
                            Ok(event) => event,
                            Err(e) => {
                                warn!("Error {},{:?}", e, msg);
                                continue;
                            }
                        };

                        level_event.debug();

                        if let Ok(mut guard) = shared.write() {
                            (*guard).set_level_event(level_event);

                            let snapshot = (*guard).get_snapshot();
                            if let Err(_) = sender.send(snapshot) {
                                error!("level_depth send Snapshot error");
                            };
                        } else {
                            error!("SharedSpot is busy");
                        }

                    };
                    Ok(())
                };

                match result {
                    Ok(_) => (),
                    Err(e) => error!("Error happen when running level_depth: {:?}", e),
                }
            }
            Ok::<(), Error>(())
        });

        Ok(receiver)
    }

    pub fn get_depth(&self) -> Option<Depth> {
        if let Ok(_) = self.status.lock() {
            Some(self.shared.write().unwrap().get_snapshot())
        } else {
            error!("BinanceSpotOrderBookPerpetualU lock is busy");
            None
        }
    }
}
