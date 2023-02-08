use std::ffi::OsStr;
use std::io::{Stdout, Write};
use std::ops::Div;
use std::path::Path;
use std::time::Duration;
use std::{collections::HashMap, fs};

use anyhow::Result;
use clap::Parser;
use crossterm::{cursor, terminal, QueueableCommand};
use futures::StreamExt;
use reqwest::{Client, Response};
use spam_can::build_request;
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
    #[arg(short, long, use_value_delimiter = true)]
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
    let config_path = Path::new(&options.config_path);
    let file = fs::read_to_string(config_path)?;

    let config: SpamConfig = match config_path.extension().and_then(OsStr::to_str) {
        Some("json") => serde_json::from_str(&file)?,
        Some("toml") => toml::from_str(&file)?,
        _ => panic!("Unsupported config file extension"),
    };

    let handles = get_test_configs(&config, &options)
        .into_iter()
        .map(|mut test_config| {
            test_config.merge_global(&config);

            test_url(test_config, config.count, options.parallelism)
        });

    for handle in handles {
        let result = handle.await;
        println!("{}", result.report());
        match result.save(&options.output_dir) {
            Ok(()) => {}
            Err(e) => println!("Error saving results for '{}': {e}", result.name,),
        }
    }

    Ok(())
}

async fn test_url(config: TestConfig, count: usize, parallelism: usize) -> TestResult {
    let start = Instant::now();
    let count = config.count.unwrap_or(count);
    let client = reqwest::Client::new();
    let f = (0..count).map(|_| {
        let config = config.clone();
        make_req(client.clone(), config)
    });

    let mut buffered = tokio_stream::iter(f).buffer_unordered(parallelism);
    let mut results = Vec::with_capacity(count);

    let mut stdout = std::io::stdout();
    let _ = writeln!(stdout, "[{}]", config.name);
    let mut complete = 0usize;
    print_progress(&mut stdout, complete, count);

    while let Some(result) = buffered.next().await {
        results.push(result);

        complete += 1;
        print_progress(&mut stdout, complete, count);
    }
    let _ = stdout.queue(cursor::MoveUp(1));
    let _ = stdout.queue(terminal::Clear(terminal::ClearType::FromCursorDown));

    TestResult::new(results, config.name.clone(), start.elapsed())
}
fn print_progress(stdout: &mut Stdout, complete: usize, count: usize) {
    static VISUAL: &str = "====================>...................";
    let length = VISUAL.len();
    let ratio = (complete as f64 / count as f64).clamp(0f64, 1f64);
    let chunks = (length as f64 * ratio).floor().div(2f64) as usize;
    let start = (length / 2) - chunks;
    let end = length - chunks;
    let _ = stdout.queue(cursor::Hide);
    let _ = stdout.queue(cursor::SavePosition);
    let _ = write!(stdout, "[{}] {complete:>8}/{count}", &VISUAL[start..end]);
    let _ = stdout.queue(cursor::RestorePosition);
    let _ = stdout.flush();
}

async fn make_req(
    client: Client,
    TestConfig {
        request,
        check_for,
        rotate_uuids,
        collect,
        latency_header,
        ..
    }: TestConfig,
) -> ResponseInfo {
    let request = build_request(client, request, rotate_uuids);

    let start = Instant::now();
    match request.send().await {
        Ok(r) => {
            let time = start.elapsed();
            let collected: HashMap<String, String> = collect
                .unwrap_or_default()
                .iter()
                .map(|v| (v.to_owned(), get_header(&r, v)))
                .collect();
            let server_latency = latency_header.map(|v| {
                let latency = get_header(&r, &v);
                let latency = latency.parse::<u64>().unwrap_or_default();
                Duration::from_millis(latency)
            });

            match check_for {
                Some(items) => match r.text().await {
                    // todo: return all that don't match
                    Ok(j) => {
                        let unmatched: Vec<_> =
                            items.into_iter().filter(|v| !j.contains(v)).collect();
                        match unmatched.len() {
                            0 => ResponseInfo::success(time, server_latency, collected),
                            _ => ResponseInfo::error(
                                time,
                                format!("Missing values {unmatched:?}"),
                                server_latency,
                                Some(collected),
                            ),
                        }
                    }
                    _ => ResponseInfo::error(
                        time,
                        "text content unavailable from response".into(),
                        server_latency,
                        Some(collected),
                    ),
                },
                None => ResponseInfo::success(time, server_latency, collected),
            }
        }
        Err(e) => ResponseInfo::error(start.elapsed(), e.to_string(), None, None),
    }
}

fn get_header(res: &Response, key: &str) -> String {
    res.headers()
        .get(key)
        .map(|h| h.to_str().unwrap_or("").to_owned())
        .unwrap_or_default()
}
