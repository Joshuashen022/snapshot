use anyhow::{anyhow, Result};
use futures_util::SinkExt;
use tokio_tungstenite::connect_async;
use tokio_tungstenite::tungstenite::Message;
use tracing::debug;
use url::Url;
use serde::de::DeserializeOwned;
use std::fmt::Debug;
use std::time::Duration;
use tokio::time::sleep;
pub use super::depth::CryptoDepth;
pub use super::ticker::CryptoTicker;
use crate::crypto::format::{GeneralRespond, heartbeat_respond, HeartbeatRequest, subscribe_message};
use crate::crypto::connection::CryptoWebSocket;

pub async fn crypto_initialize(address: &str, channel: String) -> Result<CryptoWebSocket>{

    let mut stream = socket_stream(&address).await?;
    debug!("Connect to level_address success");

    // Official suggestion
    sleep(Duration::from_millis(1000)).await;

    let message = Message::from(subscribe_message(channel.clone()));

    stream.send(message).await?;

    debug!("Subscribe to channel {} success", channel);

    Ok(stream)
}


async fn socket_stream(address: &str) -> Result<CryptoWebSocket> {
    let url = Url::parse(&address).expect("Bad URL");

    match connect_async(url).await {
        Ok((connection, _)) => Ok(connection),
        Err(e) => Err(anyhow!("{:?}", e)),
    }
}


/// Ok(true) => initialize complete
/// and this message is `StreamEvent`
///
/// Ok(false) => this message is heartbeat or response message
/// or other non-`StreamEvent`message
///
/// Err() => error happen and solve it outside
pub async fn is_live_and_keep_alive<
    Respond: DeserializeOwned + Debug
>
(
    stream: &mut CryptoWebSocket,
    message: Message
) -> Result<bool> {
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
            let order_response: Respond = serde_json::from_str(&text)?;

            debug!("Receive {:?}, initialize success", order_response);
        }
        ("subscribe", -1) => return Ok(true), // snapshot
        _ => return Err(anyhow!("Unknown respond {:?}", response)),
    }

    Ok(false)
}