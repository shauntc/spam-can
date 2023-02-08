use std::{collections::HashMap, str::FromStr};

use reqwest::{
    header::{HeaderMap, HeaderName, HeaderValue},
    Client, RequestBuilder, Url,
};

use crate::configs::RequestConfig;

pub fn build_request(
    client: Client,
    config: RequestConfig,
    rotate_uuids: Option<bool>,
) -> RequestBuilder {
    match config {
        RequestConfig::GET { url, headers } => client
            .get(configure_url(rotate_uuids, url))
            .headers(to_header_map(headers)),
        RequestConfig::POST { url, headers, body } => client
            .post(configure_url(rotate_uuids, url))
            .headers(to_header_map(headers))
            .body(body),
    }
}

fn configure_url(rotate_uuids: Option<bool>, mut url: Url) -> Url {
    match rotate_uuids {
        Some(true) => {
            let req_uuid = format!("m-{}", uuid::Uuid::new_v4().simple());
            replace_or_append_query_param(&mut url, "user", &req_uuid);
            url
        }
        _ => url,
    }
}

fn replace_or_append_query_param(url: &mut Url, name: &str, value: &str) {
    let mut query: Vec<_> = url
        .query_pairs()
        .filter(|(n, _)| n != name)
        .map(|(name, value)| (name.into_owned(), value.into_owned()))
        .collect();

    query.push((name.to_owned(), value.to_owned()));

    url.query_pairs_mut().clear().extend_pairs(&query);
}

fn to_header_map(headers: HashMap<String, String>) -> HeaderMap {
    headers
        .iter()
        .map(|(name, val)| {
            (
                HeaderName::from_str(name.as_ref()),
                HeaderValue::from_str(val.as_ref()),
            )
        })
        .filter(|(k, v)| k.is_ok() && v.is_ok())
        .map(|(k, v)| (k.unwrap(), v.unwrap()))
        .collect()
}
