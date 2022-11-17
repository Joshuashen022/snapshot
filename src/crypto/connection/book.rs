use crate::Depth;
use anyhow::{anyhow, Error, Result};
use futures_util::{Sink, SinkExt, StreamExt};
use std::sync::{Arc, Mutex, RwLock};
use tokio::sync::mpsc;
use tokio::sync::mpsc::UnboundedReceiver;
use tokio::time::{sleep, Duration};
use tokio_tungstenite::{connect_async, tungstenite::protocol::Message};
use tracing::{debug, error, info, warn};
use url::Url;

use crate::config::Config;
use crate::crypto::connection::CryptoWebSocket;
use crate::crypto::format::{
    heartbeat_respond, subscribe_message, BookEvent, BookEventStream, BookShared, GeneralRespond,
    HeartbeatRequest, OrderRespond,
};

#[derive(Clone)]
pub struct CryptoBookBookSpot {
    /// Currently not using
    status: Arc<Mutex<bool>>,
    shared: Arc<RwLock<BookShared>>,
}

impl CryptoBookBookSpot {
    pub fn new() -> Self {
        CryptoBookBookSpot {
            status: Arc::new(Mutex::new(false)),
            shared: Arc::new(RwLock::new(BookShared::new())),
        }
    }
    #[allow(unreachable_code)]
    pub fn level_depth(&self, config: Config) -> Result<UnboundedReceiver<Depth>> {
        let level_address = config.level_depth.clone().expect("level address is empty");
        let symbol = config.get_symbol().expect("spot symbol is empty");

        let shared = self.shared.clone();
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
                    info!("Connect to level_address success");

                    // Official suggestion
                    sleep(Duration::from_millis(1000)).await;

                    let channel = format!("book.{}", &symbol);

                    let message = Message::from(subscribe_message(channel.clone()));

                    match stream.send(message).await {
                        Ok(()) => (),
                        Err(e) => println!("{:?}", e),
                    };

                    info!("Subscribe to channel {} success", channel);

                    if let Ok(mut guard) = status.lock() {
                        (*guard) = true;
                    }

                    while let Ok(message) = stream.next().await.unwrap() {
                        match is_live_and_keep_alive(&mut stream, message.clone()).await {
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

                        let level_event: BookEventStream = match serde_json::from_str(&text) {
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
            Ok::<(), Error>(())
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

    pub fn get_depth(&self) -> Option<Depth> {
        if let Ok(_) = self.status.lock() {
            Some(self.shared.write().unwrap().get_snapshot())
        } else {
            error!("CryptoOrderBookSpot lock is busy");
            None
        }
    }
}

/// Ok(true) => initialize complete
/// and this message is `StreamEvent`
///
/// Ok(false) => this message is heartbeat or response message
/// or other non-`StreamEvent`message
///
/// Err() => error happen and solve it outside
async fn is_live_and_keep_alive(stream: &mut CryptoWebSocket, message: Message) -> Result<bool> {
    if !message.is_text() {
        debug!("Receive message is empty");
        return Ok(false);
    }

    let text = message.clone().into_text()?;

    let response: GeneralRespond = serde_json::from_str(&text)?;

    match (response.method.as_str(), response.id) {
        ("public/heartbeat", _) => {
            let heartbeat_request: HeartbeatRequest = serde_json::from_str(&text)?;

            debug!("Receive {:?}", heartbeat_request);

            let message = heartbeat_respond(heartbeat_request.id);

            stream.send(message).await?
        }
        ("subscribe", 1) => {
            // initialize
            let order_response: OrderRespond = serde_json::from_str(&text)?;

            debug!("Receive {:?}, initialize success", order_response);
        }
        ("subscribe", -1) => return Ok(true), // snapshot
        _ => return Err(anyhow!("Unknown respond {:?}", response)),
    }

    Ok(false)
}

#[cfg(test)]
mod tests {
    use crate::config::{Config, SymbolType};
    use crate::crypto::{BookShared, CryptoBookBookSpot};
    use crate::ExchangeType;
    use std::sync::{Arc, Mutex, RwLock};
    use tokio::runtime::Runtime;
    const LEVEL_DEPTH_URL: &str = "wss://stream.crypto.com/v2/market";

    #[test]
    fn crypto_order_book_function() {
        let config = Config {
            rest: None,
            depth: None,
            level_depth: Some(LEVEL_DEPTH_URL.to_string()),
            symbol_type: SymbolType::Spot(String::new()),
            exchange_type: ExchangeType::Crypto,
        };

        Runtime::new().unwrap().block_on(async {
            let book = CryptoBookBookSpot::new();
            let mut recv = book.level_depth(config).unwrap();

            let depth = recv.recv().await;
            assert!(depth.is_some());
        })
    }
}
