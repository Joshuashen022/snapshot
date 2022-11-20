use crate::ExchangeType;
use anyhow::{anyhow, Result};

#[derive(Clone, Debug)]
pub struct DepthConfig {
    pub rest_url: Option<String>,
    pub depth_url: Option<String>,
    pub level_depth_url: Option<String>,
    pub symbol_type: SymbolType,
    pub exchange_type: ExchangeType,
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

impl DepthConfig {
    /// Binance Spot ContractUSDT ContractCoin, Crypto Spot ContractUSDT
    pub fn is_correct(&self) -> bool {
        match (&self.symbol_type, &self.exchange_type){
            (_, ExchangeType::Binance) => true,
            (SymbolType::Spot(_), ExchangeType::Crypto) => true,
            (SymbolType::ContractUSDT(_), ExchangeType::Crypto) => true,
            _ => false
        }
    }

    pub fn is_depth(&self) -> bool {
        self.depth_url.is_some() && self.rest_url.is_some() && self.level_depth_url.is_none()
    }

    pub fn is_normal(&self) -> bool {
        self.level_depth_url.is_some() && self.depth_url.is_none() && self.rest_url.is_none()
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

    pub fn get_symbol(&self) -> String {
        match &self.symbol_type {
            SymbolType::Spot(symbol) => symbol.clone(),
            SymbolType::ContractCoin(symbol) => symbol.clone(),
            SymbolType::ContractUSDT(symbol) => symbol.clone(),
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
                    Ok(self.get_symbol())
                }
            }
            _ => Err(anyhow!(
                "Channel is unsupported for {:?}",
                self.exchange_type
            )),
        }
    }
}

#[derive(Clone, Debug)]
pub struct TickerConfig{
    pub ticker_url: String,
    pub symbol_type: SymbolType,
    pub exchange_type: ExchangeType,
}

impl TickerConfig{
    /// Binance Spot, Crypto Spot ContractUSDT
    pub fn is_correct(&self) -> bool {
        match (&self.symbol_type, &self.exchange_type){
            (SymbolType::Spot(_), ExchangeType::Binance) => true,
            (SymbolType::Spot(_), ExchangeType::Crypto) => true,
            (SymbolType::ContractUSDT(_), ExchangeType::Crypto) => true,
            _ => false
        }
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

    pub fn get_symbol(&self) -> String {
        match &self.symbol_type {
            SymbolType::Spot(symbol) => symbol.clone(),
            SymbolType::ContractCoin(symbol) => symbol.clone(),
            SymbolType::ContractUSDT(symbol) => symbol.clone(),
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
                    Ok(self.get_symbol())
                }
            }
            _ => Err(anyhow!(
                "Channel is unsupported for {:?}",
                self.exchange_type
            )),
        }
    }
}
