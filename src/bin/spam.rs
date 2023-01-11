use clap::Parser;
use tokio::{join, time::Instant};
use reqwest::{get, Response};
use anyhow::Result;

use spam_can::{TestResult, ResponseInfo, Status};

const SEG_XAP: &'static str = "https://api.msn.com/news/feed/pages/dashboard4?apikey=AEbyVYqTQAPvg4lWwMSFHaLkDzl3weRKKIDDInLQUj&fdhead=prg-pr2-na4-tre,prg-pr2-cgtrend,prg-1sw-tslt&market=en-us&ocid=winp2&timeOut=100000&user=m-18543CBC84E967AF12DD2CFE856766F3";
const NA_XAP: &'static str = "https://api.msn.com/news/feed/pages/dashboard4?apikey=AEbyVYqTQAPvg4lWwMSFHaLkDzl3weRKKIDDInLQUj&fdhead=prg-pr2-na2-tre,prg-pr2-cgtrend,prg-1sw-tslt&market=en-us&ocid=winp2&timeOut=100000&user=m-18543CBC84E967AF12DD2CFE856766F3";
const NO_XAP: &'static str = "https://api.msn.com/news/feed/pages/dashboard4?apikey=AEbyVYqTQAPvg4lWwMSFHaLkDzl3weRKKIDDInLQUj&market=en-us&ocid=winp2&timeOut=100000&user=m-18543CBC84E967AF12DD2CFE856766F3";
const NO_XAP_TRENDING: &'static str = "https://api.msn.com/news/feed/pages/dashboard4?apikey=AEbyVYqTQAPvg4lWwMSFHaLkDzl3weRKKIDDInLQUj&fdhead=prg-pr2-na-tre,prg-pr2-cgtrend,prg-1sw-tslt&market=en-us&ocid=winp2&timeOut=100000&user=m-18543CBC84E967AF12DD2CFE856766F3";

#[derive(Parser, Debug)]
struct Options {
    /// number of requests per scenario
    #[arg(short, long, default_value_t = 10)]
    count: u32,

    /// output directory
    #[arg(short, long, default_value = "out")]
    output_dir: String
}

#[tokio::main]
async fn main() -> Result<()> {
    let options = Options::parse();

    let (seg_res, na_res, no_res, nt_res) = join!(
        test_url("Segments Answers", SEG_XAP, true, options.count),
        test_url("News Answers", NA_XAP, true, options.count),
        test_url("No Flights", NO_XAP, false, options.count),
        test_url("Non-Xap Trending", NO_XAP_TRENDING, true, options.count)
    );

    println!("{}", na_res.report());
    println!("{}", seg_res.report());
    println!("{}", no_res.report());
    println!("{}", nt_res.report());

    let _ = na_res.save(&options.output_dir).map_err(|e| println!("{:?}", e));
    let _ = seg_res.save(&options.output_dir).map_err(|e| println!("{:?}", e));
    let _ = no_res.save(&options.output_dir).map_err(|e| println!("{:?}", e));
    let _ = nt_res.save(&options.output_dir).map_err(|e| println!("{:?}", e));

    Ok(())
}

async fn test_url(name: &str, url: &'static str, check_trending: bool, count: u32) -> TestResult {
    let handles = (0..count).map(|_| make_req(url, check_trending)).map(|r| tokio::spawn(r));

    let mut responses = Vec::with_capacity(handles.len());

    for handle in handles {
        match handle.await {
            Ok(res) => responses.push(res),
            Err(e) => println!("{}", e.to_string())
        }
    }

    TestResult::new(responses, name.into())
}

async fn make_req(url: &str, check_trending: bool) -> ResponseInfo {
    let time = Instant::now();
    match get(url).await {
        Ok(r) => {
            let activity_id = get_header(&r, "ddd-activityid");
            let debug_id = get_header(&r, "ddd-debugid");
            match check_trending {
                true => match r.text().await {
                    Ok(j) => {
                        match j.find("TrendingModule") {
                            Some(_) => ResponseInfo { time: time.elapsed(), debug_id, activity_id, status: Status::Success },
                            None => ResponseInfo { time: time.elapsed(), debug_id, activity_id, status: Status::Failure { reason: "no trending module".into() } }
                        }
                    },
                    _ => ResponseInfo { time: time.elapsed(), debug_id, activity_id, status: Status::Failure { reason: "failed to get text content from response".into() }}
                },
                false => ResponseInfo { time: time.elapsed(), debug_id, activity_id, status: Status::Success }
            }
        },
        Err(e) => ResponseInfo { time: time.elapsed(), debug_id: "".into(), activity_id: "".into(), status: Status::Failure { reason: e.to_string() } }
    }
}

fn get_header(res: &Response, key: &str) -> String {
    res.headers().get(key).and_then(|h| Some(h.to_str().unwrap_or("").to_owned())).unwrap_or("".to_owned())
}