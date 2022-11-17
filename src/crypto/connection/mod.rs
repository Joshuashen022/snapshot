pub mod book;
pub mod trade;

use tokio::net::TcpStream;
use tokio_tungstenite::{MaybeTlsStream, WebSocketStream};
pub type CryptoWebSocket = WebSocketStream<MaybeTlsStream<TcpStream>>;
pub use book::CryptoBookBookSpot;
pub use trade::CryptoTicker;
