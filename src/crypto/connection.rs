use crate::crypto::format::Shared;
use crate::{BinanceConnectionType, Depth};
use anyhow::{Result, Error};
use std::sync::{Arc, Mutex, RwLock};
use tokio::sync::mpsc;
use tokio::sync::mpsc::UnboundedReceiver;
use tokio::time::{sleep, Duration};
use tracing::{debug, error, info, warn};
use url::Url;
use crate::crypto::format::LevelEventStream;


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
                let result : Result<()> ={
                    if let Ok(mut guard) = status.lock() {
                        (*guard) = false;
                    }

                    info!("decoding level_event");

                    let level_event: LevelEventStream = reqwest::get(&level_address).await?.json().await?;

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
                    sleep(Duration::from_millis(100)).await;
                    Ok(())
                };

                match result {
                    Ok(_) =>(),
                    Err(e) => error!("Error happen when running level_depth: {:?}", e),
                }

            }
            Ok::<(), Error>(())
        });

        Ok(receiver)
    }
}
