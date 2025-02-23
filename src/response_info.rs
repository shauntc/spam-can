use std::{collections::HashMap, fmt::Display};

use rkyv::{Archive, Deserialize, Serialize};
use tokio::time::Duration;

#[derive(Debug, Serialize, Deserialize, Archive)]
#[archive(check_bytes)]
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
#[archive(check_bytes)]
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

impl Display for ResponseInfo {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut s = f.debug_struct("Response");

        if let Status::Failure { reason } = &self.status {
            s.field("failure", reason);
        }

        match &self.server_latency {
            Some(server_latency) => s.field(
                "time",
                &format!("{:?} (server: {:?})", self.time, server_latency),
            ),
            None => s.field("time", &self.time),
        };

        s.field("collected", &self.collected);

        s.finish()
    }
}
