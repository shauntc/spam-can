use std::collections::HashMap;

use bytecheck::CheckBytes;
use rkyv::{Archive, Deserialize, Serialize};
use tokio::time::Duration;

#[derive(Debug, Serialize, Deserialize, Archive)]
#[archive_attr(derive(CheckBytes, Debug))]
pub enum Status {
    Success,
    Failure { reason: String },
}

impl Status {
    pub fn is_success(&self) -> bool {
        match self {
            Self::Success => true,
            Self::Failure { .. } => false,
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Archive)]
#[archive_attr(derive(CheckBytes, Debug))]
pub struct ResponseInfo {
    pub time: Duration,
    pub status: Status,
    pub collected: HashMap<String, String>,
}

impl ResponseInfo {
    pub fn new() {}

    pub fn error(
        time: Duration,
        reason: String,
        collected: Option<HashMap<String, String>>,
    ) -> Self {
        Self {
            time,
            status: Status::Failure { reason },
            collected: collected.unwrap_or_default(),
        }
    }
    pub fn success(time: Duration, collected: HashMap<String, String>) -> Self {
        Self {
            time,
            status: Status::Success,
            collected,
        }
    }
}
