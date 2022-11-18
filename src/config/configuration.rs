use crate::ExchangeType;
use anyhow::{anyhow, Result};

//TODO::change to 4 address, add Ticker_url
#[derive(Clone, Debug)]
pub struct Config {
    pub rest_url: Option<String>,
    pub depth_url: Option<String>,
    /// This parameter is both used at
    /// "Level depth" and "Trade"
    pub level_trade: Option<String>,
    pub symbol_type: SymbolType,
    pub exchange_type: ExchangeType,
    pub method: Method,
}

#[derive(Clone, Debug, Copy)]
pub enum Method {
    Ticker,
    Depth,
}

#[derive(Clone, Debug, PartialOrd, PartialEq)]
pub enum SymbolType {
    Spot(String),
    ContractUSDT(String),
    ContractCoin(String),
}

impl Config {
    /// Currently we only support
    /// specific kinds of config combination
    pub fn is_correct(&self) -> bool {
        // TODO::make this work
        true
    }

    pub fn is_depth(&self) -> bool {
        self.depth_url.is_some() && self.rest_url.is_some() && self.level_trade.is_none()
    }

    pub fn is_normal(&self) -> bool {
        self.level_trade.is_some() && self.depth_url.is_none() && self.rest_url.is_none()
    }

    pub fn is_binance(&self) -> bool {
        match self.exchange_type {
            ExchangeType::Binance => true,
            _ => false,
        }
    }

    pub fn is_crypto(&self) -> bool {
        match self.exchange_type {
            ExchangeType::Crypto => true,
            _ => false,
        }
    }

    pub fn is_contract_usdt(&self) -> bool {
        match self.symbol_type {
            SymbolType::ContractUSDT(_) => true,
            _ => false,
        }
    }

    pub fn is_spot(&self) -> bool {
        match self.symbol_type {
            SymbolType::Spot(_) => true,
            _ => false,
        }
    }

    pub fn is_contract_coin(&self) -> bool {
        match self.symbol_type {
            SymbolType::ContractCoin(_) => true,
            _ => false,
        }
    }

    pub fn get_symbol(&self) -> Option<String> {
        match &self.symbol_type {
            SymbolType::Spot(symbol) => Some(symbol.clone()),
            SymbolType::ContractCoin(symbol) => Some(symbol.clone()),
            SymbolType::ContractUSDT(symbol) => Some(symbol.clone()),
        }
    }

    pub fn is_ticker(&self) -> bool {
        match self.method {
            Method::Ticker => true,
            _ => false,
        }
    }

    pub fn is_book(&self) -> bool {
        match self.method {
            Method::Depth => true,
            _ => false,
        }
    }

    /// Specialized for crypto exchange
    pub fn get_channel(&self) -> Result<String> {
        match self.exchange_type {
            ExchangeType::Crypto => {
                if self.is_contract_coin() {
                    Err(anyhow!(
                        "Crypto Channel is unsupported for {:?}",
                        self.symbol_type
                    ))
                } else {
                    self.get_symbol().ok_or(anyhow!("empty symbol"))
                }
            }
            _ => Err(anyhow!(
                "Channel is unsupported for {:?}",
                self.exchange_type
            )),
        }
    }
}
