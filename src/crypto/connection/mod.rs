pub mod book;
pub mod trade;
mod abstraction;

use tokio::net::TcpStream;
use tokio_tungstenite::{MaybeTlsStream, WebSocketStream};

pub type CryptoWebSocket = WebSocketStream<MaybeTlsStream<TcpStream>>;
pub use book::CryptoDepth;
pub use trade::CryptoTicker;

