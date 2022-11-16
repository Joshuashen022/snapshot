use crate::{Depth, Quote};
use ordered_float::OrderedFloat;
use serde::de::{SeqAccess, Visitor};
use serde::{Deserialize, Deserializer};
use std::collections::BTreeMap;
use std::fmt::{self, Debug};
use std::time::{SystemTime, UNIX_EPOCH};
use tracing::debug;

use crate::crypto::format::Quotes;
use crate::crypto::format::BookEvent;

#[derive(Deserialize, Debug)]
pub struct BookEventStream {
    /// Usually constant value `-1`
    pub id: i64,

    /// Something like: "public/get-block"
    pub method: String,

    /// Usually constant value `0`
    pub code: i64,

    pub result: BookEvent,
}
