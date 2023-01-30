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

    /// number of requests to find
    #[arg(long, short, default_value_t = 1)]
    count: u64,

    /// names of the data to extract from
    #[arg(long, short, use_value_delimiter = true)]
    names: Option<Vec<String>>,

    /// directory containing the data produced by 'spam'
    #[arg(short, long, default_value = "out/data")]
    data_dir: String,
}

fn main() -> Result<()> {
    let options = Options::parse();

    let min = Duration::from_millis(options.min_ms);
    let max = Duration::from_millis(options.max_ms);

    for result in TestResult::load_filtered(&options.data_dir, options.names.as_deref())? {
        let responses: Vec<_> = result
            .success_responses()
            .filter(|x| x.time < max && x.time > min)
            .take(options.count as usize)
            .collect();

        println!(
            "{}: (found {}/{})",
            result.name,
            responses.len(),
            options.count
        );
        if !responses.is_empty() {
            for response in responses {
                println!("{response:#?}");
            }
        } else {
            println!("Unable to find request with that range")
        }
        println!();
    }

    Ok(())
}
