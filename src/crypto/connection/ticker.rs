use crate::config::TickerConfig;
use crate::crypto::format::TickerEventStream;
use crate::Ticker;
use anyhow::Result;
use futures_util::StreamExt;
use std::sync::{Arc, Mutex};
use tokio::sync::mpsc;
use tokio::sync::mpsc::UnboundedReceiver;
use tokio::time::{sleep, Duration};

use tracing::{error, info, warn};

use super::abstraction::{crypto_initialize, is_live_and_keep_alive};
#[derive(Clone)]
pub struct CryptoTicker {
    status: Arc<Mutex<bool>>,
}

impl CryptoTicker {
    pub fn new() -> Self {
        CryptoTicker {
            status: Arc::new(Mutex::new(false)),
        }
    }

    pub fn connect(&self, config: TickerConfig) -> Result<UnboundedReceiver<Vec<Ticker>>> {
        let level_address = config.ticker_url.clone();
        let symbol = config.get_symbol();

        let status = self.status.clone();

        let (sender, receiver) = mpsc::unbounded_channel();

        let _ = tokio::spawn(async move {
            info!("Start Level Buffer maintain thread");
            loop {
                let result: Result<()> = {
                    let channel = format!("trade.{}", &symbol);
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
                        match is_live_and_keep_alive::<TickerEventStream>(
                            &mut stream,
                            message.clone(),
                        )
                        .await
                        {
                            Ok(is_alive) => {
                                if !is_alive {
                                    continue;
                                }
                            }
                            Err(e) => {
                                warn!("Decoding received message error {:?} {}", e, message);
                                continue;
                            }
                        }

                        let text = message.clone().into_text().unwrap();

                        let level_event: TickerEventStream = match serde_json::from_str(&text) {
                            Ok(event) => event,
                            Err(e) => {
                                println!("Error {}, {:?}", e, text);
                                continue;
                            }
                        };

                        if let Some(ticks) = level_event.result.add_timestamp_transform_to_ticks() {
                            if let Err(_) = sender.send(ticks) {
                                error!("Crypto Ticker send Snapshot error");
                            };
                        } else {
                            warn!("Crypto Received empty ticks")
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
}

#[cfg(test)]
mod tests {
    use crate::config::{Method, SymbolType, TickerConfig};
    use crate::crypto::connection::CryptoTicker;
    use crate::ExchangeType;
    use std::sync::{Arc, Mutex, RwLock};
    use tokio::runtime::Runtime;
    const LEVEL_DEPTH_URL: &str = "wss://stream.crypto.com/v2/market";

    #[test]
    fn crypto_ticker_function() {
        let config = TickerConfig {
            ticker_url: LEVEL_DEPTH_URL.to_string(),
            symbol_type: SymbolType::Spot(String::from("BTCUSD-PERP")),
            exchange_type: ExchangeType::Crypto,
        };

        tracing_subscriber::fmt::init();

        Runtime::new().unwrap().block_on(async {
            let ticker = CryptoTicker::new();
            let mut recv = ticker.connect(config).unwrap();

            let depth = recv.recv().await;
            assert!(depth.is_some());
        })
    }
}
