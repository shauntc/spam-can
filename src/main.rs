mod commands;
mod configs;
mod response_info;
mod test_result;

use commands::{extract, plot, spam};
pub(crate) use response_info::*;
pub(crate) use test_result::*;

use std::path::PathBuf;

use anyhow::Result;
use clap::{Parser, Subcommand};

#[derive(Subcommand, Debug)]
enum Command {
    /// run tests in a config file (ie. `spam.toml` or `spam.json`)
    Spam(spam::Options),
    /// plot histograms for the gathered results
    Plot(plot::Options),
    /// extract data from the results
    Extract(extract::Options),
}

#[derive(Parser, Debug)]
struct Options {
    #[command(subcommand)]
    command: Command,

    /// output directory
    #[arg(short, long, default_value = "out")]
    out_dir: String,

    /// config location
    #[arg(long, short, default_value = "spam.toml")]
    config_path: String,

    /// test configuration names to select
    #[arg(short, long, use_value_delimiter = true)]
    names: Option<Vec<String>>,
}

#[tokio::main]
async fn main() -> Result<()> {
    let Options {
        command,
        config_path,
        out_dir,
        names,
    } = Options::parse();
    let config_path = PathBuf::from(config_path);
    let out_dir = PathBuf::from(out_dir);
    let data_dir = out_dir.join("data");

    match command {
        Command::Spam(o) => spam::spam(o, names, config_path, data_dir).await,
        Command::Plot(o) => plot::plot(o, names, data_dir, out_dir.join("graphs")),
        Command::Extract(o) => extract::extract(o, names, data_dir),
    }
}
