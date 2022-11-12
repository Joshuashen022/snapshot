use tokio::runtime::Runtime;
use tokio::time::{sleep, Duration};

use snapshot::QuotationManager;
use snapshot::Quote;
/// const DEPTH_URL_PC: &str =      "wss://dstream.binance.com/stream?streams=btcusd_221230@depth@100ms";
/// const DEPTH_URL_PU: &str =      "wss://fstream.binance.com/stream?streams=btcusdt@depth@100ms";
/// const DEPTH_URL_SPOT: &str =    "wss://stream.binance.com:9443/ws/bnbbtc@depth@100ms";
///
///
/// const LEVEL_DEPTH_URL_PC: &str =    "wss://dstream.binance.com/stream?streams=btcusd_221230@depth20@100ms";
/// const LEVEL_DEPTH_URL_PU: &str =    "wss://fstream.binance.com/stream?streams=btcusdt@depth20@100ms";
/// const LEVEL_DEPTH_URL_SPOT: &str =  "wss://stream.binance.com:9443/ws/bnbbtc@depth20@100ms";
///
///
/// const REST_PC: &str =   "https://dapi.binance.com/dapi/v1/depth?symbol=BTCUSD_221230&limit=1000";
/// const REST_PU: &str =   "https://fapi.binance.com/fapi/v1/depth?symbol=BTCUSDT&limit=1000";
/// const REST_SPOT: &str = "https://api.binance.com/api/v3/depth?symbol=BNBBTC&limit=1000";

fn main() {
    println!("Hello");

    tracing_subscriber::fmt::init();

    Runtime::new().unwrap().block_on(async {
        let exchange = "binance";
        let pc_symbol = "BTC_USDT_221230_SWAP";
        let pu_symbol = "BTC_USDT_SWAP";
        let spot_symbol = "BTC_USDT";
        let _ = vec![pc_symbol, pu_symbol, spot_symbol];

        let symbol = pc_symbol;
        println!("using symbol {}", symbol);
        let manager1 = QuotationManager::with_snapshot(exchange, symbol, 1000);
        println!("using manager1 config {:?}", manager1.config);
        let manager1_clone = manager1.clone();
        tokio::spawn(async move {
            let mut receiver = manager1_clone.subscribe_depth();
            sleep(Duration::from_secs(2)).await;
            while let Some(message) = receiver.recv().await {
                println!(
                    "manager1 id {}, ts {}, lts {} asks {} bids {}",
                    message.id,
                    message.ts,
                    message.lts,
                    message.asks().len(),
                    message.bids().len()
                );
            }
        });

        let manager2 = QuotationManager::new(exchange, symbol);
        println!("using manager2 config {:?}", manager2.config);
        let manager2_clone = manager2.clone();
        tokio::spawn(async move {
            let mut receiver = manager2_clone.subscribe_depth();
            sleep(Duration::from_secs(2)).await;
            while let Some(message) = receiver.recv().await {
                println!(
                    "manager2 id {}, ts {}, lts {} asks {} bids {}",
                    message.id,
                    message.ts,
                    message.lts,
                    message.asks().len(),
                    message.bids().len()
                );
            }
        });

        sleep(Duration::from_secs(3)).await;
        let message = manager1.latest_depth().unwrap();
        println!(
            "snapshot1 id {}, ts {}, lts {} asks {} bids {}",
            message.id,
            message.ts,
            message.lts,
            message.asks().len(),
            message.bids().len()
        );

        let message = manager2.latest_depth().unwrap();
        println!(
            "snapshot2 id {}, ts {}, lts {} asks {} bids {}",
            message.id,
            message.ts,
            message.lts,
            message.asks().len(),
            message.bids().len()
        );

        loop {
            println!();
            println!();
            sleep(Duration::from_secs(1)).await;
        }
    });
}
