//! A task running in the background, with a state to follow and a way to pause it

use std::{
    convert::Infallible,
    sync::Arc,
    time::{Duration, Instant},
};

use axum::response::sse::Event;
use futures_util::Stream;
use serde::Serialize;
use serde_json::json;
use tokio::{
    runtime::Handle,
    sync::{Mutex as AsyncMutex, MutexGuard, watch},
    task::JoinHandle,
};
use tokio_util::sync::CancellationToken;

const UPDATE_EVERY: Duration = Duration::from_millis(250);
const SPEED_WINDOW: Duration = Duration::from_secs(1);

pub(crate) trait JobState: Clone + PartialEq + Serialize + Send + Sync + 'static {
    fn is_running(&self) -> bool;
}

pub(crate) type Running = Option<(CancellationToken, JoinHandle<()>)>;

pub(crate) struct Job<S> {
    state: watch::Sender<S>,
    running: AsyncMutex<Running>,
}

impl<S: JobState> Job<S> {
    pub fn new(state: S) -> Arc<Self> {
        Arc::new(Self {
            state: watch::Sender::new(state),
            running: AsyncMutex::new(None),
        })
    }

    pub fn state(&self) -> S {
        self.state.borrow().clone()
    }

    #[cfg(test)]
    pub fn subscribe(&self) -> watch::Receiver<S> {
        self.state.subscribe()
    }

    /// Returns false when the state was already that
    pub fn set(&self, state: S) -> bool {
        self.state.send_if_modified(|current| {
            let changed = *current != state;
            *current = state;
            changed
        })
    }

    /// Held while starting or deleting, nothing else starts meanwhile
    pub async fn lock(&self) -> MutexGuard<'_, Running> {
        self.running.lock().await
    }

    /// Returns `None` when nothing was running
    pub async fn pause(&self) -> Option<S> {
        let mut running = self.running.lock().await;
        let (cancel, task) = running.take().filter(|(_, task)| !task.is_finished())?;
        cancel.cancel();
        let _ = task.await;
        Some(self.state())
    }

    // with `pause_on_close`, closing the stream early pauses the job
    pub fn events(
        self: &Arc<Self>,
        pause_on_close: bool,
    ) -> impl Stream<Item = Result<Event, Infallible>> + use<S> {
        let mut changes = self.state.subscribe();
        // made here, the stream runs nothing until polled
        let mut guard = PauseOnClose(pause_on_close.then(|| Arc::clone(self)));
        async_stream::stream! {
            loop {
                let state = changes.borrow_and_update().clone();
                let running = state.is_running();
                yield Ok(Event::default().event("state").data(json!(state).to_string()));
                if !running || changes.changed().await.is_err() {
                    break;
                }
            }
            guard.disarm();
        }
    }
}

pub(crate) fn is_live(running: &Running) -> bool {
    running
        .as_ref()
        .is_some_and(|(_, task)| !task.is_finished())
}

struct PauseOnClose<S: JobState>(Option<Arc<Job<S>>>);

impl<S: JobState> PauseOnClose<S> {
    fn disarm(&mut self) {
        self.0 = None;
    }
}

impl<S: JobState> Drop for PauseOnClose<S> {
    fn drop(&mut self) {
        if let Some(job) = self.0.take()
            && let Ok(runtime) = Handle::try_current()
        {
            runtime.spawn(async move {
                job.pause().await;
            });
        }
    }
}

/// Download speed over the last second, handed out a few times a second
pub(crate) struct Speed {
    bytes_per_sec: u64,
    sample: Option<(Instant, u64)>,
    sent: Instant,
}

impl Speed {
    pub fn new() -> Self {
        Self {
            bytes_per_sec: 0,
            sample: None,
            sent: Instant::now(),
        }
    }

    /// The speed to send with `done`, `None` until the next update is due
    pub fn update(&mut self, done: u64) -> Option<u64> {
        let now = Instant::now();
        // the first update is what's already on disk, not speed
        let (at, from) = *self.sample.get_or_insert((now, done));
        if now - at >= SPEED_WINDOW {
            self.bytes_per_sec =
                (done.saturating_sub(from) as f64 / (now - at).as_secs_f64()) as u64;
            self.sample = Some((now, done));
        }
        if now - self.sent < UPDATE_EVERY {
            return None;
        }
        self.sent = now;
        Some(self.bytes_per_sec)
    }

    /// Holds off the next update, the state sent now stays up for a moment
    pub fn hold(&mut self) {
        self.sent = Instant::now();
    }
}
