use tokio::runtime::Runtime;
use tokio::time::{sleep, Duration};

use snapshot::TickerManager;

/// const TRADE_URL_PC: &str =    "wss://dstream.binance.com/stream?streams=btcusd_221230@trade";
/// const TRADE_URL_PU: &str =    "wss://fstream.binance.com/stream?streams=btcusdt@trade";
/// const TRADE_URL_SPOT: &str =  "wss://stream.binance.com:9443/ws/bnbbtc@trade";

fn main() {
    println!("Hello");

    tracing_subscriber::fmt::init();

    Runtime::new().unwrap().block_on(async {
        // let pc_symbol = "BTC_USD_221230_SWAP";
        // let pu_symbol = "BTC_USD_SWAP";
        // let spot_symbol = "BTC_USD";

        println!("using symbol {}", symbol);
        let manager1 = TickerManager::new("crypto", "BTC_USDT");
        println!("using manager1 config {:?}", manager1.config);
        let manager1_clone = manager1.clone();
        tokio::spawn(async move {
            let mut receiver = manager1_clone.subscribe();
            sleep(Duration::from_secs(2)).await;
            while let Some(message) = receiver.recv().await {
                println!("message {:?}", message);
            }
        });

        loop {
            println!();
            println!();
            sleep(Duration::from_secs(1)).await;
        }
    });
}
