use tokio::runtime::Runtime;
use tokio::time::{sleep, Duration};

use snapshot::connection::BinanceOrderBookType;
use snapshot::connection::BinanceConnectionType;
use snapshot::OrderbookT;

fn main(){
    println!("Hello");

    Runtime::new().unwrap().block_on(async {
        let contract_u_d = BinanceConnectionType::new_with_type(BinanceOrderBookType::PrepetualU);
        let contract_u_ld = BinanceConnectionType::new_with_type(BinanceOrderBookType::PrepetualU);

        let mut con_u_rx_d = contract_u_d.depth().unwrap();
        let mut con_u_rx_ld = contract_u_ld.level_depth().unwrap();

        tokio::spawn(async move {
            while let Some(message) = con_u_rx_d.recv().await{
                println!("receive1 {}", message.last_update_id);
            }
        });

        tokio::spawn(async move {
            while let Some(message) = con_u_rx_ld.recv().await{
                println!("receive2 {}", message.last_update_id);
            }
        });

        loop{
            println!();
            println!();
            sleep(Duration::from_secs(1)).await;
            let depth = contract_u_d.get_snapshot();

            let depth_level = contract_u_ld.get_snapshot();
            //

            let depth_time = depth.event_time;
            let depth_level_time = depth_level.event_time;
            let contains = depth.if_contains(&depth_level);

            println!("{} {}, contains? {}", depth_time, depth_level_time, contains);

            if !contains {
                let (different_bids, different_asks ) = depth.find_different(&depth_level);
                println!("bids different {}", different_bids.len());
                // println!("{:?}", different_bids);
                println!("asks different {}", different_asks.len());
                // println!("{:?}", different_asks);
            }

        }

    });
}