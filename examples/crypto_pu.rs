use tokio::runtime::Runtime;
use tokio::time::{sleep, Duration};

use snapshot::QuotationManager;

fn main() {
    println!("Hello");

    tracing_subscriber::fmt::init();

    Runtime::new().unwrap().block_on(async {
        let exchange = "crypto";
        let symbol = "BTC_USDT_SWAP";
        println!("using symbol {}", symbol);

        let manager1 = QuotationManager::with_snapshot(exchange, symbol, 10);
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
                    message.asks.len(),
                    message.bids.len()
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
                    message.asks.len(),
                    message.bids.len()
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
            message.asks.len(),
            message.bids.len()
        );

        let message = manager2.latest_depth().unwrap();
        println!(
            "snapshot2 id {}, ts {}, lts {} asks {} bids {}",
            message.id,
            message.ts,
            message.lts,
            message.asks.len(),
            message.bids.len()
        );

        loop {
            println!();
            println!();
            sleep(Duration::from_secs(1)).await;
        }
    });
}
