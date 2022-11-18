use crate::binance::BinanceTicker;
use crate::crypto::CryptoTicker;

#[derive(Clone)]
pub enum TickerConnection {
    Binance(BinanceTicker),
    Crypto(CryptoTicker),
}
