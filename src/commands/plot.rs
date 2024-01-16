use std::{
    fs,
    ops::Div,
    path::{Path, PathBuf},
};

use crate::TestResult;
use anyhow::Result;
use clap::Parser;
use ndhistogram::{axis::Uniform, ndhistogram, Histogram};
use tokio::time::Duration;

#[derive(Parser, Debug)]
pub(crate) struct Options {}

pub(crate) fn plot(
    _options: Options,
    names: Option<Vec<String>>,
    data_dir: PathBuf,
    out_dir: PathBuf,
) -> Result<()> {
    for result in TestResult::load_filtered(data_dir, names)? {
        let title = format!("{} Successes Total Latency", result.name);
        let _ = plot_histogram(
            result.success_responses().map(|res| res.time),
            &title,
            &out_dir,
        );

        let failures_title = format!("{} Failures Total Latency", result.name);
        let _ = plot_histogram(
            result.failure_responses().map(|res| res.time),
            &failures_title,
            &out_dir,
        );

        let one_s_title = format!("{} Server Latency", result.name);
        let _ = plot_histogram(
            result.responses.iter().filter_map(|r| r.server_latency),
            &one_s_title,
            &out_dir,
        );

        let req_l_title = format!("{} Infrastructure Latency", result.name);
        let _ = plot_histogram(
            result
                .responses
                .iter()
                .filter_map(|r| r.server_latency.map(|latency| r.time - latency)),
            &req_l_title,
            &out_dir,
        );
    }

    Ok(())
}

fn plot_histogram<P: AsRef<Path>>(
    data: impl Iterator<Item = Duration>,
    name: &str,
    out_dir: P,
) -> Result<()> {
    let file_path = out_dir.as_ref().join(format!("{name}.png"));
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
