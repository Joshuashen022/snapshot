use crate::format::binance_perpetual_u::{
    LevelEventPerpetualU, EventPerpetualU,
    StreamLevelEventPerpetualU, StreamEventPerpetualU,
    SharedPerpetualU, BinanceSnapshotPerpetualU
};

use crate::connection::BinanceSpotOrderBookSnapshot;

use std::collections::vec_deque::VecDeque;
use tokio_tungstenite::connect_async;
use url::Url;
use tokio::{
    time::{sleep, Duration},
    sync::mpsc::{self, UnboundedReceiver},
};
use tracing::{error, info, trace, debug};
use futures_util::StreamExt;
use anyhow::{Result, Error};
use anyhow::{bail, anyhow};
// use tokio::select;
use std::sync::{Arc, RwLock, Mutex};
use std::time::{Instant, SystemTime, UNIX_EPOCH};
use futures_util::future::err;
use tokio_tungstenite::tungstenite::Message;
// use tokio::spawn;

// const DEPTH_URL: &str = "wss://fstream.binance.com/stream?streams=btcusdt@depth@100ms";
// const LEVEL_DEPTH_URL: &str = "wss://fstream.binance.com/stream?streams=btcusdt@depth20@100ms";
// const REST: &str = "https://fapi.binance.com/fapi/v1/depth?symbol=BTCUSDT&limit=1000";
// const MAX_BUFFER: usize = 30;
const MAX_BUFFER_EVENTS: usize = 5;

pub struct BinanceSpotOrderBookPerpetualU {
    status: Arc<Mutex<bool>>,
    pub(crate) shared: Arc<RwLock<SharedPerpetualU>>,
}

impl BinanceSpotOrderBookPerpetualU {

    pub fn new() -> Self {
        BinanceSpotOrderBookPerpetualU {
            status: Arc::new(Mutex::new(false)),
            shared: Arc::new(RwLock::new(SharedPerpetualU::new()))
        }
    }

    /// acquire a order book with "depth method"
    pub fn depth(&self, rest_address: String, depth_address: String) -> Result<UnboundedReceiver<BinanceSpotOrderBookSnapshot>> {
        let shared = self.shared.clone();
        let status = self.status.clone();
        let (sender, receiver) = mpsc::unbounded_channel();
        // Thread to maintain Order Book
        let _ = tokio::spawn(async move{
            let mut default_exit = 0;
            info!("Start OrderBook thread");
            loop {
                let res : Result<()> = {
                    if let Ok(mut guard) = status.lock(){
                    
                        (*guard) = false;
                    }

                    let url = Url::parse(&depth_address).expect("Bad URL");

                    let res = connect_async(url).await;
                    let mut stream = match res{
                        Ok((stream, _)) => stream,
                        Err(e) => {
                            default_exit += 1;
                            error!("Error calling {}, {:?}", depth_address, e);
                            continue
                        },
                    };

                    info!("Calling {} success",depth_address);
                    println!("Calling {} success",depth_address);
                    let mut buffer_events = VecDeque::new();
                    while let Ok(message) = stream.next().await.unwrap(){ //
                        let event = deserialize_message(message);
                        if event.is_none(){
                            continue
                        }
                        let event = event.unwrap();

                        buffer_events.push_back(event);

                        if buffer_events.len() == MAX_BUFFER_EVENTS{
                            break
                        }
                    };

                    // Wait for a while to collect event into buffer
                    info!("Calling {} success", rest_address);
                    let snapshot: BinanceSnapshotPerpetualU = reqwest::get(&rest_address)
                        .await?
                        .json()
                        .await?;


                    trace!("Snap shot {}", snapshot.last_update_id);
                    println!("Snap shot {}", snapshot.last_update_id);
                    let mut overbook_setup = false;
                    while let Some(event) = buffer_events.pop_front() {
                        trace!(" Event {}-{}", event.first_update_id, event.last_update_id);

                        if snapshot.last_update_id > event.last_update_id  {
                            continue
                        }

                        if event.match_snapshot(snapshot.last_update_id) {
                            info!(" Found match snapshot 1");
                            let mut orderbook = shared.write().unwrap();
                            orderbook.load_snapshot(&snapshot);
                            orderbook.add_event(event);

                            overbook_setup = true;

                            break;
                        }

                        if event.first_update_id > snapshot.last_update_id {
                            error!("Rest event is not usable, need a new snap shot ");

                            break;
                        }

                    }

                    if overbook_setup {

                        while let Some(event) = buffer_events.pop_front()  {
                            let mut orderbook = shared.write().unwrap();
                            orderbook.add_event(event);
                        }

                    } else {

                        while let Ok(message) = stream.next().await.unwrap() {

                            let event = deserialize_message(message);
                            if event.is_none(){
                                continue
                            }
                            let event = event.unwrap();


                            trace!(" Event {}-{}", event.first_update_id, event.last_update_id);

                            // [E.U,..,E.u] S.u
                            if snapshot.last_update_id > event.last_update_id  {
                                continue
                            }

                            let mut orderbook = shared.write().unwrap();
                            // [E.U,..,S.u,..,E.u]
                            if event.match_snapshot(snapshot.last_update_id) {
                                info!(" Found match snapshot 2");
                                println!(" Found match snapshot 2");
                                orderbook.load_snapshot(&snapshot);
                                orderbook.add_event(event);

                                overbook_setup = true;
                                break;
                            }

                            // S.u [E.U,..,E.u]
                            if event.first_update_id > snapshot.last_update_id {
                                error!("Rest event is not usable, need a new snap shot ");

                                break;
                            }

                        }

                    }

                    if overbook_setup {
                        println!(" prepare to set lock");
                        if let Ok(mut guard) = status.lock(){
                            (*guard) = true;
                        } else {
                            println!(" set lock fail");
                        }
                    } else {

                        continue
                    }

                    info!(" Overbook initialize success, now keep listening ");
                    // Overbook initialize success
                    while let Ok(message) = stream.next().await.unwrap() {
                        let event = deserialize_message(message.clone());
                        if event.is_none(){
                            error!("Message decode error {:?}", message);
                            continue
                        }
                        let event = event.unwrap();

                        let mut orderbook = shared.write().unwrap();
                        if event.last_message_last_update_id != orderbook.id() {
                            error!("All event is not usable, need a new snap shot");

                            error!("order book {}, Event {}-{}({})",
                                    orderbook.id(), event.first_update_id, event.last_update_id, event.last_message_last_update_id);
                            break;

                        } else {
                            let f_id = event.first_update_id;
                            let l_id = event.last_update_id;
                            orderbook.add_event(event);

                            trace!("After add event {}, {} {}", orderbook.id(), f_id, l_id);
                            let snapshot = orderbook.get_snapshot();
                            if let Err(e) = sender.send(snapshot){
                                error!("depth send Snapshot error");
                            };

                        }

                    }

                    Ok(())
                };

                match res {
                    Ok(_) => (),
                    Err(e) => error!("Error happen when running code: {:?}", e),
                }

                if default_exit > 20 {
                    error!("Using default break");
                    break
                }

                default_exit += 1;
            }
            Ok::<(), Error>(())
        });

        Ok(receiver)
    }

    pub fn level_depth(&self, level_address: String) -> Result<UnboundedReceiver<BinanceSpotOrderBookSnapshot>> {
        let shared = self.shared.clone();

        // This is not actually used
        let status = self.status.clone();

        let (sender, receiver) = mpsc::unbounded_channel();


        let _ = tokio::spawn(async move {
            info!("Start Level Buffer maintain thread");
            loop{
                let url = Url::parse(&level_address).expect("Bad URL");

                let res = connect_async(url).await;
                let mut stream = match res{
                    Ok((stream, _)) => stream,
                    Err(e) => {
                        error!("Error {:?}, reconnecting {}", e, level_address);
                        continue
                    },
                };

                if let Ok(mut guard) = status.lock(){
                    (*guard) = true;
                }
                
                info!("Start Level Buffer maintain thread success");
                while let Ok(msg) = stream.next().await.unwrap(){ //
                    if !msg.is_text() {
                        error!("msg.is_text() is empty");
                        continue
                    }

                    let text = match msg.into_text(){
                        Ok(e) => e,
                        Err(e) => {
                            error!("msg.into_text {:?}", e);
                            continue
                        },
                    };

                    let level_event: StreamLevelEventPerpetualU = match serde_json::from_str(&text){
                        Ok(e) => e,
                        Err(e) => {
                            error!("Error {},{}",e, text);
                            continue
                        },
                    };
                    let level_event = level_event.data;
                    if let Ok(mut guard) = shared.write(){
                        (*guard).set_level_event(level_event);

                        let snapshot = (*guard).get_snapshot();
                        if let Err(e) = sender.send(snapshot){
                            error!("level_depth send Snapshot error");
                        };

                    }
                };
            }

        });

        Ok(receiver)
    }

    /// Get the snapshot of the current Order Book
    pub fn snapshot(&self) -> Option<BinanceSpotOrderBookSnapshot>{

        let mut current_status = false;

        if let Ok(status_guard) = self.status.lock(){
            current_status = (*status_guard).clone();

        } else {
            error!("BinanceSpotOrderBookPerpetualU lock is busy");
        }

        if current_status{
            Some(self.shared.write().unwrap().get_snapshot())
        } else{
            debug!("data is not ready");
            None
        }

    }
}


fn deserialize_message(message: Message) -> Option<EventPerpetualU>{
    if !message.is_text() {
        return None
    }

    let text = match message.into_text(){
        Ok(e) => e,
        Err(_) => return None,
    };

    let s_event: StreamEventPerpetualU = match serde_json::from_str(&text){
        Ok(e) => e,
        Err(_) => return None,
    };

    let event = s_event.data;

    Some(event)
}


