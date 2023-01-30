use std::{fs, ops::Div};

use anyhow::Result;
use clap::Parser;
use ndhistogram::{axis::Uniform, ndhistogram, Histogram};
use spam_can::TestResult;
use tokio::time::Duration;

#[derive(Parser, Debug)]
struct Options {
    /// directory containing the data produced by 'spam'
    #[arg(short, long, default_value = "out/data")]
    data_dir: String,

    /// output directory
    #[arg(short, long, default_value = "out/graphs")]
    output_dir: String,
}

fn main() -> Result<()> {
    let options = Options::parse();

    for result in TestResult::load_data(&options.data_dir)? {
        let title = format!("{} Successes", result.name);
        let _ = plot(
            result.success_responses().map(|res| res.time),
            &title,
            &options.output_dir,
        );

        let failures_title = format!("{} Failures", result.name);
        let _ = plot(
            result.failure_responses().map(|res| res.time),
            &failures_title,
            &options.output_dir,
        );

        let one_s_title = format!("{} Server Latency", result.name);
        let _ = plot(
            result.responses.iter().filter_map(|r| r.server_latency),
            &one_s_title,
            &options.output_dir,
        );

        let req_l_title = format!("{} Request Latency", result.name);
        let _ = plot(
            result
                .responses
                .iter()
                .filter_map(|r| r.server_latency.map(|latency| r.time - latency)),
            &req_l_title,
            &options.output_dir,
        );
    }

    Ok(())
}

fn plot(data: impl Iterator<Item = Duration>, name: &str, out_dir: &str) -> Result<()> {
    let file_path = format!("{out_dir}/{name}.png");
    let _ = fs::create_dir_all(out_dir);

    let buckets = 80usize;

    let start = 0u32;
    let end = 2000u32;
    let step = (end - start).div(buckets as u32);

    let mut histogram = ndhistogram!(Uniform::with_step_size(buckets, start, step));
    let data_ms = data.map(|t| t.as_millis() as u32).collect::<Vec<u32>>();
    data_ms.iter().for_each(|t| histogram.fill(t));

    let highest_count = histogram
        .iter()
        .map(|v| (*v.value as u32))
        .max()
        .ok_or_else(|| anyhow::anyhow!("no max??"))?;

    let std_dev =
        std_deviation(&data_ms).ok_or_else(|| anyhow::anyhow!("unable to calculate std_dev"))?;
    let avg = mean(&data_ms).ok_or_else(|| anyhow::anyhow!("unable to calculate avg"))?;

    use plotters::prelude::*;

    let root_drawing_area = BitMapBackend::new(&file_path, (2000, 1000)).into_drawing_area();

    root_drawing_area.fill(&WHITE)?;
    let dev_message = format!("std_dev: {std_dev}");
    let avg_msg = format!("mean: {avg}");

    let style = ("Segoe UI", 30).into_text_style(&root_drawing_area);
    root_drawing_area.draw_text(&dev_message, &style, (1700, 100))?;
    root_drawing_area.draw_text(&avg_msg, &style, (1700, 130))?;

    let mut ctx = ChartBuilder::on(&root_drawing_area)
        .caption(name, ("Segoe UI", 30))
        .set_label_area_size(LabelAreaPosition::Left, 40)
        .set_label_area_size(LabelAreaPosition::Bottom, 40)
        .build_cartesian_2d((start..end).into_segmented(), 0..(highest_count + 1))?;

    ctx.configure_mesh().draw()?;

    ctx.draw_series(
        Histogram::vertical(&ctx).margin(10).data(
            histogram
                .iter()
                .filter_map(|v| v.bin.start().map(|bin| (bin, (*v.value) as u32))),
        ),
    )?;

    Ok(())
}

fn mean(data: &[u32]) -> Option<f32> {
    let sum = data.iter().sum::<u32>() as f32;
    let count = data.len();

    match count {
        positive if positive > 0 => Some(sum / count as f32),
        _ => None,
    }
}

fn std_deviation(data: &[u32]) -> Option<f32> {
    match (mean(data), data.len()) {
        (Some(data_mean), count) if count > 0 => {
            let variance = data
                .iter()
                .map(|value| {
                    let diff = data_mean - (*value as f32);

                    diff * diff
                })
                .sum::<f32>()
                / count as f32;

            Some(variance.sqrt())
        }
        _ => None,
    }
}
