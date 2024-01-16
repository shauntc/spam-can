use crossterm::{cursor, terminal, QueueableCommand};
use futures::StreamExt;
use std::io::{Stdout, Write};
use std::ops::Div;
use tokio::time::Instant;

use super::{Cancellation, SpamService};
use crate::{configs::ResolvedConfig, test_result::TestResult};

pub(crate) async fn test(config: ResolvedConfig, cancellation: Cancellation) -> TestResult {
    let start = Instant::now();
    let count = config.count;
    let mut service =
        SpamService::new(config.clone(), cancellation).expect("unable to build spam service");
    let mut stream = service.run_test().await;

    // let mut buffered = tokio_stream::iter(stream).buffer_unordered(parallelism);

    let mut results = Vec::with_capacity(count);

    let mut stdout = std::io::stdout();
    let _ = writeln!(stdout, "[{}]", config.name);
    let mut complete = 0usize;

    print_progress(&mut stdout, complete, count);

    while let Some(result) = stream.next().await {
        results.push(result);

        complete += 1;
        if complete % 10 == 0 {
            print_progress(&mut stdout, complete, count);
        }
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
