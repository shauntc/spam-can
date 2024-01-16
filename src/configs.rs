use reqwest::Url;
use serde::{de, Deserialize, Deserializer};
use std::{collections::HashMap, time::Duration};

mod defaults {
    use std::time::Duration;

    pub fn count() -> usize {
        10
    }
    pub fn rotate_uuids() -> bool {
        false
    }
    pub fn timeout() -> Duration {
        Duration::from_secs(30)
    }
}
#[derive(Debug, Deserialize, Clone)]
#[serde(deny_unknown_fields)]
pub struct GlobalConfig {
    /// values to check for in all responses
    pub check_for: Option<Vec<String>>,

    /// the number of requests unless specified in a specific test config
    #[serde(default = "defaults::count")]
    pub count: usize,

    /// whether to use a random uuid in the `user` request param for each request
    #[serde(default = "defaults::rotate_uuids")]
    pub rotate_uuids: bool,

    /// header values to collect from all responses
    pub collect: Option<Vec<String>>,

    /// default timeout for all tests
    #[serde(
        default = "defaults::timeout",
        deserialize_with = "deserialize::duration"
    )]
    pub timeout: Duration,

    /// max rps default for each test
    pub max_rps: usize,

    pub max_concurrent: usize,
}

#[derive(Debug, Deserialize, Clone)]
#[serde(deny_unknown_fields)]
pub struct SpamConfig {
    #[serde(flatten)]
    pub global: GlobalConfig,
    pub test_configs: Vec<TestConfig>,
}

#[derive(Debug, Deserialize, Clone)]
#[serde(deny_unknown_fields)]
pub struct TestConfig {
    pub name: String,
    pub request: RequestConfig,
    /// items to check for in the request text
    pub check_for: Option<Vec<String>>,
    /// override for the number of requests to this url
    pub count: Option<usize>,
    /// whether to use a random uuid in the `user` request param for each request
    pub rotate_uuids: Option<bool>,
    /// header values to collect from responses
    pub collect: Option<Vec<String>>,
    /// the key of a header that contains how long the server used processing the request in ms
    pub latency_header: Option<String>,
    /// timeout for all requests
    #[serde(deserialize_with = "deserialize::duration_option", default)]
    pub timeout: Option<Duration>,
    pub max_rps: Option<usize>,
    pub max_concurrent: Option<usize>,
}

#[derive(Clone)]
pub(crate) struct ResolvedConfig {
    pub name: String,
    pub request: RequestConfig,
    pub check_for: Option<Vec<String>>,
    pub count: usize,
    pub rotate_uuids: bool,
    pub collect: Option<Vec<String>>,
    pub latency_header: Option<String>,
    pub timeout: Duration,
    pub max_rps: usize,
    pub max_concurrent: usize,
}

#[derive(Debug, Deserialize, Clone)]
#[serde(tag = "method", deny_unknown_fields)]
pub enum RequestConfig {
    #[serde(alias = "get", alias = "GET")]
    Get {
        #[serde(deserialize_with = "deserialize::url")]
        url: Url,
        #[serde(default)]
        headers: HashMap<String, String>,
    },
    #[serde(alias = "post", alias = "POST")]
    Post {
        #[serde(deserialize_with = "deserialize::url")]
        url: Url,
        #[serde(default)]
        headers: HashMap<String, String>,
        #[serde(default)]
        body: String,
    },
}

impl TestConfig {
    pub(crate) fn resolve(self, global: &GlobalConfig) -> ResolvedConfig {
        let Self {
            name,
            request,
            check_for,
            count,
            rotate_uuids,
            collect,
            latency_header,
            timeout,
            max_rps,
            max_concurrent,
        } = self;

        let check_for = match (check_for, &global.check_for) {
            (Some(l), Some(g)) => Some([l, g.clone()].concat()),
            (None, None) => None,
            (None, x) => x.clone(),
            (x, None) => x,
        };

        let collect = match (collect, &global.collect) {
            (Some(l), Some(g)) => Some([l, g.clone()].concat()),
            (None, None) => None,
            (None, x) => x.clone(),
            (x, None) => x,
        };
        let count = count.unwrap_or(global.count);
        let rotate_uuids = rotate_uuids.unwrap_or(global.rotate_uuids);
        let timeout = timeout.unwrap_or(global.timeout);
        let max_rps = max_rps.unwrap_or(global.max_rps);
        let max_concurrent = max_concurrent.unwrap_or(global.max_concurrent);

        ResolvedConfig {
            name,
            request,
            check_for,
            count,
            rotate_uuids,
            collect,
            latency_header,
            timeout,
            max_rps,
            max_concurrent,
        }
    }
}

mod deserialize {
    use super::*;

    #[derive(Deserialize)]
    struct UrlComponents {
        base_url: String,
        query: Option<HashMap<String, String>>,
    }

    #[derive(Deserialize)]
    #[serde(untagged)]
    enum UrlOrParts {
        Url(Url),
        Parts(UrlComponents),
    }
    pub fn url<'de, D: Deserializer<'de>>(d: D) -> Result<Url, D::Error> {
        match UrlOrParts::deserialize(d)? {
            UrlOrParts::Url(url) => Ok(url),
            UrlOrParts::Parts(parts) => match parts.query {
                Some(query) => Url::parse_with_params(&parts.base_url, query.iter()),
                None => Url::parse(&parts.base_url),
            }
            .map_err(de::Error::custom),
        }
    }

    #[derive(Deserialize)]
    #[serde(untagged)]
    enum DurationOrMs {
        D(Duration),
        Ms(u64),
    }

    pub fn duration<'de, D: Deserializer<'de>>(d: D) -> Result<Duration, D::Error> {
        match DurationOrMs::deserialize(d)? {
            DurationOrMs::D(duration) => Ok(duration),
            DurationOrMs::Ms(ms) => Ok(Duration::from_millis(ms)),
        }
    }
    pub fn duration_option<'de, D: Deserializer<'de>>(d: D) -> Result<Option<Duration>, D::Error> {
        match Option::<DurationOrMs>::deserialize(d)? {
            Some(DurationOrMs::D(duration)) => Ok(Some(duration)),
            Some(DurationOrMs::Ms(ms)) => Ok(Some(Duration::from_millis(ms))),
            None => Ok(None),
        }
    }
}
