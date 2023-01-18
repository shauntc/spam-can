use std::fmt;

use reqwest::Url;
use serde::{de, Deserialize, Deserializer};

fn uuid_rotate_default() -> bool {
    false
}
fn count_default() -> usize {
    10
}

#[derive(Debug, Deserialize, Clone)]
pub struct SpamConfig {
    pub check_for: Option<Vec<String>>,
    #[serde(default = "count_default")]
    pub count: usize,
    #[serde(default = "uuid_rotate_default")]
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
