use std::fmt;

use reqwest::Url;
use serde::{de, Deserialize, Deserializer};

#[derive(Debug, Deserialize, Clone)]
pub struct SpamConfig {
    pub check_for: Option<Vec<String>>,
    #[serde(default = "defaults::count")]
    pub count: usize,
    #[serde(default = "defaults::rotate_uuids")]
    pub rotate_uuids: bool,
    pub collect: Option<Vec<String>>,

    pub test_configs: Vec<TestConfig>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct TestConfig {
    pub name: String,
    #[serde(deserialize_with = "deserialize_url")]
    pub url: Url,
    pub flights: Option<String>,
    pub check_for: Option<Vec<String>>,
    pub count: Option<usize>,
    pub rotate_uuids: Option<bool>,
    pub collect: Option<Vec<String>>,
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

fn deserialize_url<'de, D: Deserializer<'de>>(d: D) -> Result<Url, D::Error> {
    struct UrlVisitor;
    impl<'de> de::Visitor<'de> for UrlVisitor {
        type Value = Url;
        fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
            formatter.write_str("a valid url")
        }
        fn visit_str<E: de::Error>(self, v: &str) -> Result<Self::Value, E> {
            Url::parse(v).map_err(E::custom)
        }
    }

    d.deserialize_any(UrlVisitor)
}

mod defaults {
    pub fn count() -> usize {
        10
    }
    pub fn rotate_uuids() -> bool {
        false
    }
}
