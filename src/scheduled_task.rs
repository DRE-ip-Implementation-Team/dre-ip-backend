use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;
use std::task::{Context, Poll};

use chrono::{DateTime, Utc};
use rocket::tokio::{
    self,
    sync::Notify,
    task::{JoinError, JoinHandle},
    time::Duration,
};

/// A task scheduled for a specific point in the future.
/// It will automatically execute at that point, or can be cancelled or triggered early.
pub struct ScheduledTask<T> {
    task_handle: JoinHandle<T>,
    wait_handle: JoinHandle<()>,
    signal: Arc<Notify>,
}

impl<T> ScheduledTask<T>
where
    T: Send + 'static,
{
    /// Schedule the given task to execute at time `run_at`.
    /// If `run_at` is in the past, the task will execute immediately.
    pub fn new<Fut>(task: Fut, run_at: DateTime<Utc>) -> Self
    where
        Fut: Future<Output = T> + Send + 'static,
    {
        // Create the synchronisation signal.
        let signal = Arc::new(Notify::new());

        // Schedule the task to wait on the signal.
        let task_signal = signal.clone();
        let task_handle = tokio::spawn(async move {
            task_signal.notified().await;
            task.await
        });

        // Spawn another task to give the signal at the appropriate time.
        let sleep_duration = datetime_to_duration(run_at);
        let wait_signal = signal.clone();
        let wait_handle = tokio::spawn(async move {
            tokio::time::sleep(sleep_duration).await;
            wait_signal.notify_one();
        });

        Self {
            task_handle,
            wait_handle,
            signal,
        }
    }

    /// Cancel the task. Returns true iff it had already completed before we could cancel it.
    pub async fn cancel(self) -> bool {
        self.task_handle.abort();
        self.wait_handle.abort();
        self.task_handle.await.is_ok()
    }

    /// Trigger the task now instead of waiting till the original time.
    pub fn trigger_now(&self) {
        self.wait_handle.abort();
        self.signal.notify_one();
    }
}

/// Implement `Future` for `ScheduledTask` so we can directly `await` it.
impl<T> Future for ScheduledTask<T> {
    type Output = Result<T, JoinError>;

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        Pin::new(&mut self.task_handle).poll(cx)
    }
}

/// Convert a `DateTime` into a duration from the current instant.
/// A `DateTime` in the past will produce a duration of zero.
fn datetime_to_duration(datetime: DateTime<Utc>) -> Duration {
    let target_timestamp = datetime.timestamp_millis();
    let now_timestamp = Utc::now().timestamp_millis();
    let time_diff = u64::try_from(target_timestamp - now_timestamp).unwrap_or(0);
    Duration::from_millis(time_diff)
}
