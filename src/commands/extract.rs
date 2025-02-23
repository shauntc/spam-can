use std::{path::PathBuf, time::Duration};

use crate::{ResponseInfo, TestResult};
use anyhow::{anyhow, Result};
use clap::{Parser, Subcommand};

#[derive(Subcommand, Debug)]
pub(crate) enum Command {
    Range {
        /// minimum request length to find in ms
        #[arg(long)]
        min_ms: u64,

        // maximum request length to find in ms
        #[arg(long)]
        max_ms: u64,

        /// number of requests to find
        #[arg(long, short, default_value_t = 1)]
        count: u64,
    },
    Percentiles,
    Failures {
        /// number of requests to find
        #[arg(long, short, default_value_t = 1)]
        count: usize,
    },
}

#[derive(Parser, Debug)]
pub(crate) struct Options {
    #[command(subcommand)]
    command: Command,
}

pub(crate) fn extract(
    Options { command }: Options,
    names: Option<Vec<String>>,
    data_dir: PathBuf,
) -> Result<()> {
    match command {
        Command::Range {
            min_ms,
            max_ms,
            count,
        } => {
            if count == 0 {
                return Err(anyhow!("Count cannot be 0"));
            }
            let min = Duration::from_millis(min_ms);
            let max = Duration::from_millis(max_ms);

            for result in TestResult::load_filtered(&data_dir, names)? {
                let responses: Vec<_> = result
                    .success_responses()
                    .filter(|x| x.time < max && x.time > min)
                    .take(count as usize)
                    .collect();

                println!("{}: (found {}/{})", result.name, responses.len(), count);
                if !responses.is_empty() {
                    for response in responses {
                        println!("{response:#}");
                    }
                } else {
                    println!("Unable to find request with that range")
                }
                println!();
            }
        }
        Command::Percentiles => {
            for mut result in TestResult::load_filtered(&data_dir, names)? {
                result.responses.sort_unstable_by_key(|r| r.time);
                println!("{}:", result.name);
                println!("  P75: {}", percentile_time(&result.responses, 0.75));
                println!("  P95: {}", percentile_time(&result.responses, 0.95));
                println!("  P99: {}", percentile_time(&result.responses, 0.99));
                println!("  P99.5: {}", percentile_time(&result.responses, 0.995));
                println!("  P99.9: {}", percentile_time(&result.responses, 0.999));
            }
        }
        Command::Failures { count } => {
            if count == 0 {
                return Err(anyhow!("Count cannot be 0"));
            }
            for result in TestResult::load_filtered(&data_dir, names)? {
                let responses: Vec<_> = result.failure_responses().take(count).collect();

                println!("{}: (found {}/{})", result.name, responses.len(), count);
                if !responses.is_empty() {
                    for response in responses {
                        println!("{response:#}");
                    }
                } else {
                    println!("No failed requests")
                }
                println!();
            }
        }
    }

    Ok(())
}

fn percentile_time(responses: &[ResponseInfo], ratio: f64) -> String {
    let n_total = responses.len();
    let n = (n_total as f64 * ratio).floor() as usize;
    let subset = &responses[n..];
    let count = n_total - n;

    match subset.first().map(|v| v.time) {
        Some(t) => format!("{t:?} ({count}/{n_total})"),
        None => format!("unable to calculate ({count}/{n_total})"),
    }
}
