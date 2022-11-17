pub(crate) mod binance;
pub(crate) mod crypto;

use api::config;

pub mod api;
pub use api::{Depth, ExchangeType, QuotationManager, Quote};
pub use api::{OrderDirection, Tick};
pub use config::{match_up, Config, Connection, SymbolType};

#[cfg(test)]
mod tests {
    #[test]
    fn manager_builder_works() {
        use crate::QuotationManager;

        let exchange = "binance";
        let exchange2 = "crypto";
        let pc_symbol = "btcusd_221230_swap";
        let pu_symbol = "btcusdt_swap";
        let spot_symbol = "bnbbtc";
        let limit = 1000;

        let _ = QuotationManager::with_snapshot(exchange, pc_symbol, limit);
        let _ = QuotationManager::with_snapshot(exchange, pu_symbol, limit);
        let _ = QuotationManager::with_snapshot(exchange, spot_symbol, limit);

        let _ = QuotationManager::new(exchange, pc_symbol);
        let _ = QuotationManager::new(exchange, pu_symbol);
        let _ = QuotationManager::new(exchange, spot_symbol);

        let _ = QuotationManager::with_snapshot(exchange2, pc_symbol, limit);
        let _ = QuotationManager::with_snapshot(exchange2, pu_symbol, limit);
        let _ = QuotationManager::with_snapshot(exchange2, spot_symbol, limit);

        let _ = QuotationManager::new(exchange2, pc_symbol);
        let _ = QuotationManager::new(exchange2, pu_symbol);
        let _ = QuotationManager::new(exchange2, spot_symbol);
    }

    #[test]
    #[should_panic]
    fn manager_builder_wrong_exchange() {
        use crate::QuotationManager;

        let wrong_exchange = "binanc";
        let pc_symbol = "btcusd_221230_swap";
        let limit = 1000;

        let _ = QuotationManager::with_snapshot(wrong_exchange, pc_symbol, limit);
    }

    #[test]
    #[should_panic]
    fn manager_builder_wrong_symbol() {
        use crate::QuotationManager;

        let wrong_exchange = "binance";
        let pc_symbol = "btcusd_221230swap";
        let limit = 1000;

        let _ = QuotationManager::with_snapshot(wrong_exchange, pc_symbol, limit);
    }
}
