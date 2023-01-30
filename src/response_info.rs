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
    pub server_latency: Option<Duration>,
    pub collected: HashMap<String, String>,
}

impl ResponseInfo {
    pub fn error(
        time: Duration,
        reason: String,
        server_latency: Option<Duration>,
        collected: Option<HashMap<String, String>>,
    ) -> Self {
        Self {
            time,
            status: Status::Failure { reason },
            server_latency,
            collected: collected.unwrap_or_default(),
        }
    }
    pub fn success(
        time: Duration,
        server_latency: Option<Duration>,
        collected: HashMap<String, String>,
    ) -> Self {
        Self {
            time,
            status: Status::Success,
            server_latency,
            collected,
        }
    }
}
