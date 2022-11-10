use tokio::runtime::Runtime;
use tokio::time::{sleep, Duration};

use snapshot::get_depth_snapshot;
use snapshot::subscribe_depth;
use snapshot::subscribe_depth_snapshot;

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


fn main(){
    println!("Hello");

    use tracing_subscriber;
    tracing_subscriber::fmt::init();

    Runtime::new().unwrap().block_on(async {
        let exchange = "binance";
        let pc_symbol = "btcusd_221230_swap";
        let pu_symbol = "btcusdt_swap";
        let spot_symbol = "bnbbtc";
        let _ = vec![pc_symbol,pu_symbol,spot_symbol];

        let limit = 1000;
        let symbol = spot_symbol;
        println!("using symbol {}", symbol);
        tokio::spawn(async move {
            let mut receiver = subscribe_depth_snapshot(exchange, symbol, limit).unwrap();
            while let Some(message) = receiver.recv().await
            {
                println!("receive1 {}", message.last_update_id);
            }
        });

        tokio::spawn(async move {
            while let Some(message) = subscribe_depth(exchange, symbol)
                .unwrap().recv().await
            {
                println!("receive2 {}", message.last_update_id);
            }
        });

        let snapshot = get_depth_snapshot(exchange, symbol, limit).unwrap();
        println!("snapshot {}", snapshot.last_update_id);
        loop{
            sleep(Duration::from_secs(1)).await;
        }
    });
}