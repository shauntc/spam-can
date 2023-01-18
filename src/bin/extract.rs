use std::time::Duration;

use anyhow::Result;
use clap::Parser;
use spam_can::TestResult;

#[derive(Parser)]
struct Options {
    /// minimum request length to find in ms
    #[arg(long)]
    min_ms: u64,

    // maximum request length to find in ms
    #[arg(long)]
    max_ms: u64,

    /// directory containing the data produced by 'spam'
    #[arg(short, long, default_value = "out/data")]
    data_dir: String,
}

fn main() -> Result<()> {
    let options = Options::parse();

    let min = Duration::from_millis(options.min_ms);
    let max = Duration::from_millis(options.max_ms);

    for result in TestResult::load_data(&options.data_dir)? {
        let res = result
            .success_responses()
            .find(|x| x.time < max && x.time > min);
        if let Some(res) = res {
            println!("{}:", result.name);
            println!("{:#?}", res);
            return Ok(());
        }
    }

    Err(anyhow::anyhow!("Unable to find request with that range"))
}
