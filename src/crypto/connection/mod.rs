mod abstraction;
pub mod depth;
pub mod ticker;

use tokio::net::TcpStream;
use tokio_tungstenite::{MaybeTlsStream, WebSocketStream};

pub type CryptoWebSocket = WebSocketStream<MaybeTlsStream<TcpStream>>;
pub use depth::CryptoDepth;
pub use ticker::CryptoTicker;
