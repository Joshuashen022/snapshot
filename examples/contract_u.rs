use tokio::runtime::Runtime;

use snapshot::connection::BinanceOrderBookType;
use snapshot::connection::BinanceConnectionType;

fn main(){
    println!("Hello");

    Runtime::new().unwrap().block_on(async {
        let contract_u = BinanceConnectionType::new_with_type(BinanceOrderBookType::PrepetualU);

        let mut con_u_rx_d = contract_u.depth().unwrap();
        let mut con_u_rx_ld = contract_u.level_depth().unwrap();

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

    });
}