use std::fs::OpenOptions;
use std::io::Write;
use tokio::runtime::Runtime;
use tokio::time::{sleep, Duration};

use snapshot::QuotationManager;
fn main() {
    println!("Hello");

    tracing_subscriber::fmt::init();

    Runtime::new().unwrap().block_on(async {
        let exchange = "binance";
        let pc_symbol = "BTC_USDT_221230_SWAP";
        let pu_symbol = "BTC_USDT_SWAP";
        let spot_symbol = "BTC_USDT";
        let _ = vec![pc_symbol, pu_symbol, spot_symbol];

        let symbol = spot_symbol;
        println!("using symbol {}", symbol);

        let manager1 = QuotationManager::with_snapshot(exchange, symbol, 1000);
        let manager1_clone = manager1.clone();
        println!("using manager1 config {:?}", manager1.config);

        tokio::spawn(async move {
            let mut receiver = manager1_clone.subscribe_depth();
            let mut file = OpenOptions::new();
            let mut reader = file.create(true).append(true).open("depth.cache").unwrap();
            sleep(Duration::from_secs(2)).await;
            while let Some(message) = receiver.recv().await {
                println!(
                    "manager1 id {}, ts {}, lts {} asks {} bids {}",
                    message.id,
                    message.ts,
                    message.lts,
                    message.asks.len(),
                    message.bids.len()
                );

                let raw = serde_json::to_string(&message).unwrap();
                let raw = format!("{}\n",raw);
                reader.write_all(raw.as_bytes()).unwrap_or(());
            }
        });


        let manager2 = QuotationManager::new(exchange, symbol);
        println!("using manager2 config {:?}", manager2.config);
        let manager2_clone = manager2.clone();
        tokio::spawn(async move {
            let mut receiver = manager2_clone.subscribe_depth();
            let mut file = OpenOptions::new();
            let mut reader = file.create(true).append(true).open("normal.cache").unwrap();
            sleep(Duration::from_secs(2)).await;
            while let Some(message) = receiver.recv().await {
                println!(
                    "manager2 id {}, ts {}, lts {} asks {} bids {}",
                    message.id,
                    message.ts,
                    message.lts,
                    message.asks.len(),
                    message.bids.len()
                );
                let raw = serde_json::to_string(&message).unwrap();
                let raw = format!("{}\n",raw);
                reader.write_all(raw.as_bytes()).unwrap_or(());
            }
        });
        sleep(Duration::from_secs(3)).await;

        loop {
            println!();
            println!();
            sleep(Duration::from_secs(1)).await;
            // let depth = manager1.snapshot();
            // let normal = manager2.snapshot();
            // if normal.is_none() || depth.is_none(){
            //     println!("depth_level {}, depth {}", normal.is_none(), depth.is_none());
            //     continue
            // }
            // let depth = depth.unwrap();
            // let normal = normal.unwrap();
            // let depth_time = depth.send_time;
            // let depth_level_time = normal.send_time;
            // let contains = depth.if_contains(&normal);
            //
            // println!("{} {}, contains? {}", depth_time, depth_level_time, contains);
            // if !contains {
            //     let (different_bids, different_asks ) = depth.find_different(&normal);
            //     println!("bids different {}", different_bids.len());
            //     // println!("{:?}", different_bids);
            //     println!("asks different {}", different_asks.len());
            //     // println!("{:?}", different_asks);
            // }

        }
    });
}
