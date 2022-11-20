use crate::binance::format::ticker::EventTicker;
use crate::{TickerConfig, Ticker};
use anyhow::{Error, Result};
use futures_util::StreamExt;
use std::sync::{Arc, Mutex};
use std::time::Duration;
use tokio::sync::mpsc;
use tokio::sync::mpsc::UnboundedReceiver;
use tokio::time::sleep;
use tokio_tungstenite::connect_async;
use tracing::{error, info, warn};
use url::Url;

#[derive(Clone)]
pub struct BinanceTicker {
    status: Arc<Mutex<bool>>,
}

impl BinanceTicker {
    pub fn new() -> Self {
        Self {
            status: Arc::new(Mutex::new(false)),
        }
    }

    #[allow(unreachable_code)]
    pub fn connect(&self, config: TickerConfig) -> Result<UnboundedReceiver<Vec<Ticker>>> {
        let level_address = config.ticker_url.clone();
        let status = self.status.clone();
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
                            sleep(Duration::from_millis(25)).await;
                            continue;
                        }
                    };
                    info!("Connect to {} success", &level_address);

                    if let Ok(mut guard) = status.lock() {
                        (*guard) = true;
                    }

                    while let Ok(message) = stream.next().await.unwrap() {
                        if !message.is_text() {
                            warn!("message is empty");
                            continue;
                        }

                        let text = match message.clone().into_text() {
                            Ok(e) => e,
                            Err(e) => {
                                warn!("message.into_text {:?}", e);
                                continue;
                            }
                        };

                        let response: EventTicker = match serde_json::from_str(&text) {
                            Ok(response) => response,
                            Err(e) => {
                                warn!("Error {}, {:?}", e, message);
                                continue;
                            }
                        };

                        if let Some(ticks) = response.add_timestamp_transform_to_ticks() {
                            if let Err(_) = sender.send(ticks) {
                                error!("Binance Ticker send Snapshot error");
                            };
                        } else {
                            warn!("Binance Received empty ticks")
                        }
                    }
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
}

#[cfg(test)]
mod tests {
    use crate::binance::connection::ticker::BinanceTicker;
    use crate::config::{DepthConfig, Method, SymbolType};
    use crate::ExchangeType;
    use std::sync::{Arc, Mutex, RwLock};
    use tokio::runtime::Runtime;

    const TICKER_URL: &str = "wss://stream.binance.com:9443/ws/bnbbtc@trade";

    #[test]
    fn binance_ticker_function() {
        let config = DepthConfig {
            rest_url: None,
            depth_url: None,
            level_depth_url: Some(TICKER_URL.to_string()),
            symbol_type: SymbolType::Spot(String::from("BTCUSD-PERP")),
            exchange_type: ExchangeType::Binance,
            method: Method::Ticker,
        };

        tracing_subscriber::fmt::init();

        Runtime::new().unwrap().block_on(async {
            let ticker = BinanceTicker::new();
            let mut recv = ticker.connect(config).unwrap();

            let depth = recv.recv().await;
            assert!(depth.is_some());
        })
    }
}
