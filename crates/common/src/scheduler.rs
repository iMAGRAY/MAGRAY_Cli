use std::sync::Arc;
use std::time::Duration;
use tokio::sync::{broadcast, RwLock};

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct JobId(pub &'static str);

#[derive(Debug, Clone)]
pub struct ScheduledJob {
    pub id: JobId,
    pub interval: Duration,
}

#[derive(Clone)]
pub struct Scheduler {
    inner: Arc<RwLock<Inner>>,    
}

struct Inner {
    shutdown_tx: broadcast::Sender<()>,
}

impl Scheduler {
    pub fn new() -> Self {
        let (tx, _rx) = broadcast::channel(8);
        Self { inner: Arc::new(RwLock::new(Inner { shutdown_tx: tx })) }
    }

    pub async fn spawn_periodic<F, Fut>(&self, job: ScheduledJob, mut f: F) where
        F: FnMut() -> Fut + Send + 'static,
        Fut: std::future::Future<Output = ()> + Send + 'static,
    {
        let rx = { self.inner.read().await.shutdown_tx.subscribe() };
        tokio::spawn(run_periodic(job, rx, f));
    }

    pub async fn shutdown(&self) {
        let tx = { self.inner.read().await.shutdown_tx.clone() };
        let _ = tx.send(());
    }
}

async fn run_periodic<F, Fut>(job: ScheduledJob, mut shutdown_rx: broadcast::Receiver<()>, mut f: F)
where
    F: FnMut() -> Fut + Send + 'static,
    Fut: std::future::Future<Output = ()> + Send + 'static,
{
    let mut ticker = tokio::time::interval(job.interval);
    loop {
        tokio::select! {
            _ = ticker.tick() => {
                f().await;
            }
            _ = shutdown_rx.recv() => {
                break;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};

    #[tokio::test]
    async fn periodic_job_runs_and_stops() {
        let sched = Scheduler::new();
        let counter = Arc::new(AtomicUsize::new(0));
        let c2 = counter.clone();
        let job = ScheduledJob { id: JobId("test.job"), interval: Duration::from_millis(20) };
        sched.spawn_periodic(job, move || {
            let c = c2.clone();
            async move {
                c.fetch_add(1, Ordering::Relaxed);
            }
        }).await;

        tokio::time::sleep(Duration::from_millis(75)).await;
        sched.shutdown().await;
        let n = counter.load(Ordering::Relaxed);
        assert!(n >= 2, "expected at least 2 ticks, got {}", n);
    }
}