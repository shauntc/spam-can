mod cancellation;
mod create_request;
mod test;
mod test_client;

pub(crate) use cancellation::*;
use test_client::SpamService;

use std::ffi::OsStr;
use std::fs;
use std::path::PathBuf;

use anyhow::Result;
use clap::Parser;

use crate::configs::SpamConfig;
use crate::configs::TestConfig;

#[derive(Parser, Debug)]
pub(crate) struct Options {}

pub(crate) async fn spam(
    _options: Options,
    names: Option<Vec<String>>,
    config_path: PathBuf,
    output_dir: PathBuf,
) -> Result<()> {
    let file = fs::read_to_string(&config_path)?;

    let config: SpamConfig = match &config_path.extension().and_then(OsStr::to_str) {
        Some("json") => serde_json::from_str(&file)?,
        Some("toml") => toml::from_str(&file)?,
        _ => panic!("Unsupported config file extension"),
    };

    let cancellation = Cancellation::new();
    tokio::spawn(watch_cancellation(cancellation.clone()));

    let test_configs: Vec<TestConfig> = match names {
        Some(names) => names
            .into_iter()
            .filter_map(|n| config.test_configs.iter().find(|t| t.name == n).cloned())
            .collect(),
        None => config.test_configs,
    };

    let handles = test_configs
        .into_iter()
        .map(|test_config| test_config.resolve(&config.global))
        .map(|test_config| test::test(test_config, cancellation.clone()));

    for handle in handles {
        let result = handle.await;
        println!("{}", result.report());
        if let Err(e) = result.save(&output_dir) {
            println!("Error saving results for '{}': {e}", result.name);
        }

        if cancellation.is_canceled() {
            break;
        }
    }

    Ok(())
}
