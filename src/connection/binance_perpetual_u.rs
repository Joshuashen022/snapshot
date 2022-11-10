use crate::format::binance_perpetual_u::{
    LevelEventPerpetualU, EventPerpetualU,
    StreamLevelEventPerpetualU, StreamEventPerpetualU,
    SharedPerpetualU, BinanceSnapshotPerpetualU
};
use crate::Depth;
use crate::connection::BinanceOrderBookSnapshot;

use std::collections::vec_deque::VecDeque;
use tokio_tungstenite::connect_async;
use url::Url;
use tokio::{
    time::{sleep, Duration},
    sync::mpsc::{self, UnboundedReceiver},
};
use tracing::{error, info, trace, debug, warn};
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

#[derive(Clone)]
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
    pub fn depth(&self, rest_address: String, depth_address: String) -> Result<UnboundedReceiver<Depth>> {
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

                    info!("Successfully connected to {}", depth_address);

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
                    let snapshot: BinanceSnapshotPerpetualU = reqwest::get(&rest_address)
                        .await?
                        .json()
                        .await?;

                    info!("Successfully connected to {}", rest_address);

                    debug!("Snap shot {}", snapshot.last_update_id);

                    let mut overbook_setup = false;
                    while let Some(event) = buffer_events.pop_front() {
                        debug!(" Event {}-{}", event.first_update_id, event.last_update_id);

                        if snapshot.last_update_id > event.last_update_id  {
                            continue
                        }

                        if event.match_snapshot(snapshot.last_update_id) {

                            info!(" Found match snapshot at buffer");

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
                        debug!("Emptying events in buffer");
                        while let Some(event) = buffer_events.pop_front()  {
                            let mut orderbook = shared.write().unwrap();
                            orderbook.add_event(event);
                        }

                    } else {

                        info!(" Try to wait new events for out snapshot");

                        while let Ok(message) = stream.next().await.unwrap() {

                            let event = deserialize_message(message);
                            if event.is_none(){
                                continue
                            }
                            let event = event.unwrap();


                            debug!(" Event {}-{}", event.first_update_id, event.last_update_id);

                            // [E.U,..,E.u] S.u
                            if snapshot.last_update_id > event.last_update_id  {
                                continue
                            }

                            let mut orderbook = shared.write().unwrap();
                            // [E.U,..,S.u,..,E.u]
                            if event.match_snapshot(snapshot.last_update_id) {

                                info!(" Found match snapshot with new event");

                                orderbook.load_snapshot(&snapshot);
                                orderbook.add_event(event);

                                overbook_setup = true;
                                break;
                            }

                            // S.u [E.U,..,E.u]
                            if event.first_update_id > snapshot.last_update_id {
                                warn!("Rest event is not usable, need a new snap shot ");

                                break;
                            }

                        }

                    }

                    if overbook_setup {

                        if let Ok(mut guard) = status.lock(){
                            (*guard) = true;
                        }
                    } else {

                        continue
                    }

                    info!(" Overbook initialize success, now keep listening ");

                    // Overbook initialize success
                    while let Ok(message) = stream.next().await.unwrap() {
                        let event = deserialize_message(message.clone());
                        if event.is_none(){
                            warn!("Message decode error {:?}", message);
                            continue
                        }
                        let event = event.unwrap();

                        debug!("receive event {}-{}({}) ts: {}",
                            event.first_update_id, event.last_update_id, event.last_message_last_update_id,
                            event.event_time,
                        );

                        let mut orderbook = shared.write().unwrap();
                        if event.last_message_last_update_id != orderbook.id() {
                            warn!("All event is not usable, need a new snap shot");

                            debug!("order book {}, Event {}-{}({})",
                                    orderbook.id(), event.first_update_id, event.last_update_id, event.last_message_last_update_id);
                            break;

                        } else {
                            let f_id = event.first_update_id;
                            let l_id = event.last_update_id;
                            orderbook.add_event(event);

                            debug!("After add event {}, {} {}", orderbook.id(), f_id, l_id);
                            let snapshot = orderbook.get_snapshot().depth();
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
            error!("OrderBook thread stopped");
            Ok::<(), Error>(())
        });

        Ok(receiver)
    }

    pub fn level_depth(&self, level_address: String) -> Result<UnboundedReceiver<Depth>> {
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

                info!("Successfully connected to {}", level_address);

                if let Ok(mut guard) = status.lock(){
                    (*guard) = true;
                }

                info!("Level Overbook initialize success, now keep listening ");

                while let Ok(msg) = stream.next().await.unwrap(){ //
                    if !msg.is_text() {
                        warn!("msg is empty");
                        continue
                    }

                    let text = match msg.clone().into_text(){
                        Ok(e) => e,
                        Err(e) => {
                            warn!("msg.into_text {:?}", e);
                            continue
                        },
                    };

                    let level_event: StreamLevelEventPerpetualU = match serde_json::from_str(&text){
                        Ok(e) => e,
                        Err(e) => {
                            warn!("Error {},{:?}", e, msg);
                            continue
                        },
                    };
                    let level_event = level_event.data;

                    debug!("receive level_event {}-{}({}) ts: {}",
                        level_event.first_update_id, level_event.last_update_id, level_event.last_message_last_update_id,
                        level_event.event_time,
                    );

                    if let Ok(mut guard) = shared.write(){
                        (*guard).set_level_event(level_event);

                        let snapshot = (*guard).get_snapshot().depth();
                        if let Err(e) = sender.send(snapshot){
                            error!("level_depth send Snapshot error");
                        };

                    }else{
                        error!("SharedSpot is busy");
                    }
                };
            }

        });

        Ok(receiver)
    }

    /// Get the snapshot of the current Order Book
    pub fn snapshot(&self) -> Option<Depth>{

        let mut current_status = false;

        if let Ok(status_guard) = self.status.lock(){
            current_status = (*status_guard).clone();

        } else {
            error!("BinanceSpotOrderBookPerpetualU lock is busy");
        }

        if current_status{
            Some(self.shared.write().unwrap().get_snapshot().depth())
        } else{
            debug!("data is not ready");
            None
        }

    }

    pub(crate) fn set_symbol(&mut self, symbol: String) -> Result<()>{
        {
            match self.shared.clone().write(){
                Ok(mut shared) => {
                    (*shared).symbol = symbol;
                    Ok(())
                },
                Err(e) => Err(anyhow!("{:?}", e)),
            }
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


