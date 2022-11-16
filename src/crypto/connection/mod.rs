pub mod book;
pub mod order;

use tokio::net::TcpStream;
use tokio_tungstenite::{MaybeTlsStream, WebSocketStream};
pub use order::CryptoOrderBookSpot;
pub type CryptoWebSocket = WebSocketStream<MaybeTlsStream<TcpStream>>;