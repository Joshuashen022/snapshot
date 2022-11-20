use crate::Depth;
use anyhow::{Error, Result};
use futures_util::StreamExt;
use std::sync::{Arc, Mutex, RwLock};
use tokio::sync::mpsc;
use tokio::sync::mpsc::UnboundedReceiver;
use tokio::time::{sleep, Duration};
use tracing::{error, info, warn};

use crate::config::DepthConfig;
use crate::crypto::format::{
    DepthEventStream, DepthShared, OrderRespond,
};

use super::abstraction::{is_live_and_keep_alive, crypto_initialize};

#[derive(Clone)]
pub struct CryptoDepth {
    /// Currently not using
    status: Arc<Mutex<bool>>,
    shared: Arc<RwLock<DepthShared>>,
}

impl CryptoDepth {
    pub fn new() -> Self {
        CryptoDepth {
            status: Arc::new(Mutex::new(false)),
            shared: Arc::new(RwLock::new(DepthShared::new())),
        }
    }

    pub fn level_depth(&self, config: DepthConfig) -> Result<UnboundedReceiver<Depth>> {
        let level_address = config.level_depth_url.clone().expect("level address is empty");
        let symbol = config.get_symbol();

        let shared = self.shared.clone();
        let status = self.status.clone();

        let (sender, receiver) = mpsc::unbounded_channel();

        let _ = tokio::spawn(async move {
            info!("Start Level Buffer maintain thread");
            loop {
                let result: Result<()> = {
                    let channel = format!("book.{}", &symbol);

                    let mut stream = match crypto_initialize(&level_address, channel).await {
                        Ok(connection) => connection,
                        Err(e) => {
                            warn!("connection error {:?}", e);
                            sleep(Duration::from_millis(25)).await;
                            continue;
                        }
                    };

                    if let Ok(mut guard) = status.lock() {
                        (*guard) = true;
                    }

                    while let Ok(message) = stream.next().await.unwrap() {
                        match is_live_and_keep_alive::<OrderRespond>(&mut stream, message.clone()).await {
                            Ok(is_alive) => {
                                if !is_alive {
                                    continue;
                                }
                            }
                            Err(e) => {
                                warn!("Decoding received message error {:?}", e);
                                continue;
                            }
                        }

                        let text = message.clone().into_text().unwrap();

                        let level_event: DepthEventStream = match serde_json::from_str(&text) {
                            Ok(event) => event,
                            Err(e) => {
                                println!("Error {}, {:?}", e, text);
                                continue;
                            }
                        };

                        if let Ok(mut guard) = shared.write() {
                            (*guard).set_level_event(level_event);

                            let snapshot = (*guard).get_snapshot();
                            if let Err(_) = sender.send(snapshot) {
                                error!("level_depth send Snapshot error");
                            };
                        } else {
                            error!("SharedSpot is busy");
                        }
                    }
                    Ok(())
                };

                match result {
                    Ok(_) => (),
                    Err(e) => error!("Error happen when running level_depth: {:?}", e),
                }
            }
        });

        Ok(receiver)
    }

    pub fn snapshot(&self) -> Option<Depth> {
        let mut current_status = false;

        if let Ok(status_guard) = self.status.lock() {
            current_status = (*status_guard).clone();
        } else {
            error!("CryptoOrderBookSpot lock is busy");
        }

        if current_status {
            Some(self.shared.write().unwrap().get_snapshot())
        } else {
            None
        }
    }
}


#[cfg(test)]
mod tests {
    use crate::config::Method;
    use crate::config::{DepthConfig, SymbolType};
    use crate::crypto::{DepthShared, CryptoDepth};
    use crate::ExchangeType;
    use std::sync::{Arc, Mutex, RwLock};
    use tokio::runtime::Runtime;

    const LEVEL_DEPTH_URL: &str = "wss://stream.crypto.com/v2/market";

    #[test]
    fn crypto_order_book_function() {
        let config = DepthConfig {
            rest_url: None,
            depth_url: None,
            level_depth_url: Some(LEVEL_DEPTH_URL.to_string()),
            symbol_type: SymbolType::Spot(String::new()),
            exchange_type: ExchangeType::Crypto,
            method: Method::Depth,
        };

        Runtime::new().unwrap().block_on(async {
            let book = CryptoDepth::new();
            let mut recv = book.level_depth(config).unwrap();

            let depth = recv.recv().await;
            assert!(depth.is_some());
        })
    }
}
