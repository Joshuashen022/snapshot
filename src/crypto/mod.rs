// https://api.crypto.com/v2/public/get-book?instrument_name=BTC_USDT&depth=10
//
//
// https://api.crypto.com/v2/{method}
// https://uat-api.3ona.co/v2/{method}
mod format;
mod connection;

pub use connection::CryptoOrderBookSpot;
pub use format::Shared;