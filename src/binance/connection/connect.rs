use tokio::net::TcpStream;
use tokio_tungstenite::{connect_async, tungstenite, MaybeTlsStream, WebSocketStream};
// use tungstenite::{Message, WebSocket};
use url::Url;

pub type BinanceWebSocket = WebSocketStream<MaybeTlsStream<TcpStream>>;

pub async fn socket_stream(address: &str) -> Result<BinanceWebSocket, String>{
    let url = Url::parse(&address).expect("Bad URL");

    match connect_async(url).await{
        Ok((connection, _)) => Ok(connection),
        Err(e) => Err(format!("{:?}", e))
    }

}