pub mod book;
pub mod trade;

use tokio::net::TcpStream;
use tokio_tungstenite::{connect_async, MaybeTlsStream, WebSocketStream};
use url::Url;

pub type CryptoWebSocket = WebSocketStream<MaybeTlsStream<TcpStream>>;
pub use book::CryptoBookBookSpot;
pub use trade::CryptoTicker;

pub async fn socket_stream(address: &str) -> Result<CryptoWebSocket, String> {
    let url = Url::parse(&address).expect("Bad URL");

    match connect_async(url).await {
        Ok((connection, _)) => Ok(connection),
        Err(e) => Err(format!("{:?}", e)),
    }
}

