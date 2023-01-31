use reqwest::Url;
use serde::{de, Deserialize, Deserializer};
use std::collections::HashMap;

mod defaults {
    pub fn count() -> usize {
        10
    }
    pub fn rotate_uuids() -> bool {
        false
    }
}

#[derive(Debug, Deserialize, Clone)]
#[serde(deny_unknown_fields)]
pub struct SpamConfig {
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

    pub test_configs: Vec<TestConfig>,
}

#[derive(Debug, Deserialize, Clone)]
#[serde(deny_unknown_fields)]
pub struct TestConfig {
    pub name: String,
    #[serde(deserialize_with = "deserialize::url")]
    pub url: Url,
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
}

impl TestConfig {
    pub fn merge_global(&mut self, global: &SpamConfig) {
        if let Some(check_for) = &global.check_for {
            let v = self.check_for.get_or_insert_with(Vec::new);
            v.append(&mut check_for.clone());
        }
        self.count.get_or_insert(global.count);
        self.rotate_uuids.get_or_insert(global.rotate_uuids);

        if let Some(collect) = &global.collect {
            let v = self.collect.get_or_insert_with(Vec::new);
            v.append(&mut collect.clone());
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
}
