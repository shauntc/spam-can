use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};

#[derive(Clone)]
pub(crate) struct Cancellation(Arc<AtomicBool>);
impl Cancellation {
    pub(crate) fn new() -> Self {
        Self(Arc::new(AtomicBool::new(false)))
    }
    pub(crate) fn is_canceled(&self) -> bool {
        self.0.load(Ordering::Relaxed)
    }
    fn cancel(self) {
        self.0.swap(true, Ordering::Relaxed);
    }
}

pub(crate) async fn watch_cancellation(cancellation: Cancellation) {
    let ctrl_c = async {
        tokio::signal::ctrl_c()
            .await
            .expect("failed to install Ctrl+C handler");
    };

    #[cfg(unix)]
    let terminate = async {
        tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
            .expect("failed to install signal handler")
            .recv()
            .await;
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();
    tokio::select! {
        _ = ctrl_c => {},
        _ = terminate => {},
    }
    println!("shutdown initiated, waiting for requests to complete...");
    cancellation.cancel();
}
