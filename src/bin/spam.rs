use std::{collections::HashMap, fs};

use anyhow::Result;
use clap::Parser;
use futures::StreamExt;
use reqwest::{get, Response, Url};
use tokio::time::Instant;

use spam_can::{
    configs::{SpamConfig, TestConfig},
    ResponseInfo, TestResult,
};

#[derive(Parser, Debug)]
struct Options {
    /// number of parallel requests to make   NOTE: each test is done in series, this only applies to the requests in a test
    #[arg(short, long, default_value_t = 10)]
    parallelism: usize,

    /// output directory
    #[arg(short, long, default_value = "out/data")]
    output_dir: String,

    #[arg(long, short, default_value = "spam.toml")]
    config_path: String,

    /// test configuration names to run
    #[arg(short, long)]
    tests: Option<Vec<String>>,
}

fn get_test_configs(config: &SpamConfig, options: &Options) -> Vec<TestConfig> {
    if let Some(test_names) = &options.tests {
        return test_names
            .iter()
            .map(
                |test_name| match config.test_configs.iter().find(|v| test_name.eq(&v.name)) {
                    Some(test_config) => test_config.clone(),
                    _ => panic!("Name '{test_name}' doesn't correspond to a test config"),
                },
            )
            .collect();
    } else {
        config.test_configs.iter().map(Clone::clone).collect()
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    let options = Options::parse();
    let file = fs::read_to_string(&options.config_path)?;

    let config: SpamConfig = toml::from_str(&file)?;

    let handles = get_test_configs(&config, &options)
        .into_iter()
        .map(|mut test_config| {
            if let Some(g_collect) = &config.collect {
                let collect = test_config.collect.get_or_insert(Vec::new());
                for item in g_collect {
                    collect.push(item.clone());
                }
            }
            test_config.rotate_uuids.get_or_insert(config.rotate_uuids);
            test_config
        })
        .map(|test_config| test_url(test_config, config.count, options.parallelism));

    for handle in handles {
        let time = Instant::now();
        let result = handle.await;
        let duration = time.elapsed();
        println!("[{duration:?}] {}", result.report());
        match result.save(&options.output_dir) {
            Ok(()) => {}
            Err(e) => println!("Error saving results for '{}': {e}", result.name,),
        }
    }

    Ok(())
}

async fn test_url(config: TestConfig, count: usize, parallelism: usize) -> TestResult {
    let count = config.count.unwrap_or(count);
    let f = (0..count).map(|_| {
        let mut url = config.url.clone();
        if let Some(true) = config.rotate_uuids {
            let req_uuid = format!("m-{}", uuid::Uuid::new_v4().simple());
            let query: Vec<_> = url
                .query_pairs()
                .filter(|(name, _)| name != "user")
                .map(|(name, value)| (name.into_owned(), value.into_owned()))
                .collect();
            url.query_pairs_mut()
                .clear()
                .extend_pairs(&query)
                .append_pair("user", &req_uuid);
        }
        let check_for = config.check_for.clone();
        let collect = config.collect.clone();
        tokio::spawn(make_req(url.clone(), check_for, collect))
    });

    let results = tokio_stream::iter(f)
        .buffer_unordered(parallelism)
        .collect::<Vec<_>>()
        .await
        .into_iter()
        .filter_map(|r| match r {
            Ok(result) => Some(result),
            Err(e) => {
                println!("Join Error: {e:?}");
                None
            }
        })
        .collect::<Vec<_>>();

    TestResult::new(results, config.name)
}

async fn make_req(
    url: Url,
    check_for: Option<Vec<String>>,
    collect: Option<Vec<String>>,
) -> ResponseInfo {
    let start = Instant::now();
    match get(url).await {
        Ok(r) => {
            let time = start.elapsed();
            let collected: HashMap<String, String> = collect
                .unwrap_or(Vec::new())
                .iter()
                .map(|v| (v.to_owned(), get_header(&r, v)))
                .collect();

            match check_for {
                Some(items) => match r.text().await {
                    // todo: return all that don't match
                    Ok(j) => {
                        let unmatched: Vec<_> =
                            items.into_iter().filter(|v| !j.contains(v)).collect();
                        match unmatched.len() {
                            0 => ResponseInfo::success(time, collected),
                            _ => ResponseInfo::error(
                                time,
                                format!("Missing values {unmatched:?}"),
                                Some(collected),
                            ),
                        }
                    }
                    _ => ResponseInfo::error(
                        time,
                        "text content unavailable from response".into(),
                        None,
                    ),
                },
                None => ResponseInfo::success(time, collected),
            }
        }
        Err(e) => ResponseInfo::error(start.elapsed(), e.to_string(), None),
    }
}

fn get_header(res: &Response, key: &str) -> String {
    res.headers()
        .get(key)
        .and_then(|h| Some(h.to_str().unwrap_or("").to_owned()))
        .unwrap_or("".to_owned())
}
