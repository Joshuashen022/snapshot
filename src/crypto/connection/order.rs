
use crate::Depth;
use anyhow::{Error, Result, anyhow};
use std::sync::{Arc, Mutex, RwLock};
use futures_util::{Sink, SinkExt, StreamExt};
use tokio::sync::mpsc;
use tokio::time::{Duration, sleep};
use tokio::sync::mpsc::UnboundedReceiver;
use tokio_tungstenite::{connect_async, tungstenite::protocol::Message};
use tracing::{debug, error, info, warn};
use url::Url;

use crate::crypto::format::BookShared;
use crate::crypto::format::OrderRespond;
use crate::crypto::format::subscribe_message;
use crate::crypto::format::GeneralRespond;
use crate::crypto::format::heartbeat_respond;
use crate::crypto::format::HeartbeatRequest;
use crate::crypto::connection::CryptoWebSocket;
use crate::crypto::format::BookEventStream;
use crate::config::Config;
use crate::crypto::format::BookEvent;

#[derive(Clone)]
pub struct CryptoOrderBookSpot {
    /// Currently not using
    status: Arc<Mutex<bool>>,
    shared: Arc<RwLock<BookShared>>,
}

impl CryptoOrderBookSpot {
    pub fn new() -> Self {
        CryptoOrderBookSpot {
            status: Arc::new(Mutex::new(false)),
            shared: Arc::new(RwLock::new(BookShared::new())),
        }
    }
    #[allow(unreachable_code)]
    pub fn level_depth(&self, config: Config) -> Result<UnboundedReceiver<Depth>> {
        let level_address = config.level_depth.unwrap();
        let shared = self.shared.clone();

        let (sender, receiver) = mpsc::unbounded_channel();

        let _ = tokio::spawn(async move {
            info!("Start Level Buffer maintain thread");
            loop {
                let result: Result<()> = {

                    const LEVEL_DEPTH_URL: &str = "wss://stream.crypto.com/v2/market";
                    let url = Url::parse(LEVEL_DEPTH_URL).expect("Bad URL");
                    let mut stream = match connect_async(url).await {
                        Ok((connection, _)) => connection,
                        Err(e) => {
                            warn!("connection error {:?}", e);
                            sleep(Duration::from_millis(25)).await;
                            continue ;
                        }
                    };
                    info!("Connect to level_address success");

                    // Official suggestion
                    sleep(Duration::from_millis(1000)).await;

                    let channel = String::from("book.BTCUSD-PERP");
                    let message = Message::from(subscribe_message(channel.clone()));
                    match stream.send(message).await{
                        Ok(()) => (),
                        Err(e) => println!("{:?}",e ),
                    };

                    info!("Subscribe to channel {} success", channel);

                    while let Ok(message) = stream.next().await.unwrap() {

                        match is_live_and_keep_alive(&mut stream, message.clone()).await{
                            Ok(is_alive) => {
                                if !is_alive{
                                    continue
                                }
                            }
                            Err(e) =>{
                                warn!("Decoding received message error {:?}", e);
                                continue
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

    pub fn snapshot(&self) -> Option<Depth>{
        let mut current_status = false;

        if let Ok(status_guard) = self.status.lock() {
            current_status = (*status_guard).clone();
        } else {
            error!("BinanceSpotOrderBookPerpetualU lock is busy");
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
            error!("BinanceSpotOrderBookPerpetualU lock is busy");
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
async fn is_live_and_keep_alive(stream:&mut CryptoWebSocket, message: Message) -> Result<bool>{

    let text = message.clone().into_text()?;

    let response: GeneralRespond = serde_json::from_str(&text)?;

    match (response.method.as_str(), response.id){
        ("public/heartbeat", _) => {
            let heartbeat_request: HeartbeatRequest = serde_json::from_str(&text)?;

            debug!("Receive {:?}", heartbeat_request);

            let message = heartbeat_respond(heartbeat_request.id);

            stream.send(message).await?
        },
        ("subscribe", 1)=> { // initialize
            let order_response: OrderRespond = serde_json::from_str(&text)?;

            debug!("Receive {:?}, initialize success", order_response);
        },
        ("subscribe", -1)=> return Ok(true), // snapshot
        _ => return Err(anyhow!("Unknown respond {:?}", response)),
    }

    Ok(false)
}