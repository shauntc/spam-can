use std::time::Duration;

use rkyv::{Serialize, Deserialize, Archive};
use bytecheck::CheckBytes;

#[derive(Debug, Serialize, Deserialize, Archive)]
#[archive_attr(derive(CheckBytes, Debug))]
pub enum Status {
    Success,
    Failure { reason: String }
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
    pub debug_id: String,
    pub activity_id: String,
    pub status: Status,
}