use crate::crypto::format::Shared;
use crate::{BinanceConnectionType, Depth};
use anyhow::Result;
use std::sync::{Arc, Mutex, RwLock};
use tokio::sync::mpsc::UnboundedReceiver;

#[derive(Clone)]
pub struct CryptoOrderBookSpot {
    status: Arc<Mutex<bool>>,
    shared: Arc<RwLock<Shared>>,
}

impl CryptoOrderBookSpot {
    pub fn new() -> Self {
        CryptoOrderBookSpot {
            status: Arc::new(Mutex::new(false)),
            shared: Arc::new(RwLock::new(Shared::new())),
        }
    }

    pub fn level_depth(&self, level_address: String) -> Result<UnboundedReceiver<Depth>> {
        unimplemented!()
    }
}
