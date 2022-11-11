use std::sync::{Arc, Mutex, RwLock};
use crate::crypto::format::Shared;


#[derive(Clone)]
pub struct CryptoOrderBookSpot {
    status: Arc<Mutex<bool>>,
    shared: Arc<RwLock<Shared>>,
}

impl CryptoOrderBookSpot{
    pub fn new() ->Self{
        CryptoOrderBookSpot {
            status: Arc::new(Mutex::new(false)),
            shared: Arc::new(RwLock::new(Shared::new()))
        }
    }
}