
pub(crate) mod binance;
pub(crate) mod crypto;

pub(crate) mod api;
pub(crate) mod config;

pub(crate) use config::{
    DepthConnection, TickerConnection,
    SymbolType, get_config_from
};

pub use api::{
    DepthManager, TickerManager,
    Depth, Ticker, Quote,
    ExchangeType, OrderDirection
};

pub use config::Config;

#[cfg(test)]
mod tests {
    #[test]
    fn manager_builder_works() {
        use crate::DepthManager;

        let exchange = "binance";
        let exchange2 = "crypto";
        let pc_symbol = "btcusd_221230_swap";
        let pu_symbol = "btcusdt_swap";
        let spot_symbol = "bnbbtc";
        let limit = 1000;

        let _ = DepthManager::with_snapshot(exchange, pc_symbol, limit);
        let _ = DepthManager::with_snapshot(exchange, pu_symbol, limit);
        let _ = DepthManager::with_snapshot(exchange, spot_symbol, limit);

        let _ = DepthManager::new(exchange, pc_symbol);
        let _ = DepthManager::new(exchange, pu_symbol);
        let _ = DepthManager::new(exchange, spot_symbol);

        let _ = DepthManager::with_snapshot(exchange2, pc_symbol, limit);
        let _ = DepthManager::with_snapshot(exchange2, pu_symbol, limit);
        let _ = DepthManager::with_snapshot(exchange2, spot_symbol, limit);

        let _ = DepthManager::new(exchange2, pc_symbol);
        let _ = DepthManager::new(exchange2, pu_symbol);
        let _ = DepthManager::new(exchange2, spot_symbol);
    }

    #[test]
    #[should_panic]
    fn manager_builder_wrong_exchange() {
        use crate::DepthManager;

        let wrong_exchange = "binanc";
        let pc_symbol = "btcusd_221230_swap";
        let limit = 1000;

        let _ = DepthManager::with_snapshot(wrong_exchange, pc_symbol, limit);
    }

    #[test]
    #[should_panic]
    fn manager_builder_wrong_symbol() {
        use crate::DepthManager;

        let wrong_exchange = "binance";
        let pc_symbol = "btcusd_221230swap";
        let limit = 1000;

        let _ = DepthManager::with_snapshot(wrong_exchange, pc_symbol, limit);
    }
}
