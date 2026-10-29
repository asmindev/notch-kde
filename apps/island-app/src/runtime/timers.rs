use island_core::{AppEvent, TimerId};
use std::collections::HashMap;
use std::time::Duration;
use tokio::runtime::Handle;
use tokio::sync::mpsc::UnboundedSender;
use tokio::task::JoinHandle;

pub struct TimerScheduler {
    runtime: Handle,
    event_tx: UnboundedSender<AppEvent>,
    handles: HashMap<TimerId, JoinHandle<()>>,
}

impl TimerScheduler {
    pub fn new(runtime: Handle, event_tx: UnboundedSender<AppEvent>) -> Self {
        Self {
            runtime,
            event_tx,
            handles: HashMap::new(),
        }
    }

    pub fn schedule(&mut self, id: TimerId, delay: Duration, generation: u64) {
        self.cancel(id);
        let event_tx = self.event_tx.clone();
        let handle = self.runtime.spawn(async move {
            tokio::time::sleep(delay).await;
            let _ = event_tx.send(AppEvent::TimerFired { id, generation });
        });
        self.handles.insert(id, handle);
    }

    pub fn cancel(&mut self, id: TimerId) {
        if let Some(handle) = self.handles.remove(&id) {
            handle.abort();
        }
    }

    pub fn cancel_all(&mut self) {
        for (_, handle) in self.handles.drain() {
            handle.abort();
        }
    }

    pub fn shutdown(&mut self) {
        self.cancel_all();
    }
}

impl Drop for TimerScheduler {
    fn drop(&mut self) {
        self.cancel_all();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::sync::mpsc;

    fn scheduler() -> (TimerScheduler, mpsc::UnboundedReceiver<AppEvent>) {
        let (event_tx, event_rx) = mpsc::unbounded_channel();
        (TimerScheduler::new(Handle::current(), event_tx), event_rx)
    }

    async fn recv(rx: &mut mpsc::UnboundedReceiver<AppEvent>) -> AppEvent {
        tokio::time::timeout(Duration::from_secs(1), rx.recv())
            .await
            .expect("timer did not fire")
            .expect("event channel closed")
    }

    #[tokio::test]
    async fn schedule_then_fire() {
        let (mut scheduler, mut event_rx) = scheduler();
        scheduler.schedule(TimerId::Welcome, Duration::from_millis(5), 7);

        assert_eq!(
            recv(&mut event_rx).await,
            AppEvent::TimerFired {
                id: TimerId::Welcome,
                generation: 7,
            }
        );
    }

    #[tokio::test]
    async fn reschedule_cancels_previous_timer() {
        let (mut scheduler, mut event_rx) = scheduler();
        scheduler.schedule(TimerId::Alert, Duration::from_millis(100), 1);
        scheduler.schedule(TimerId::Alert, Duration::from_millis(5), 2);

        assert_eq!(
            recv(&mut event_rx).await,
            AppEvent::TimerFired {
                id: TimerId::Alert,
                generation: 2,
            }
        );
        assert!(
            tokio::time::timeout(Duration::from_millis(150), event_rx.recv())
                .await
                .is_err()
        );
    }

    #[tokio::test]
    async fn cancel_prevents_fire() {
        let (mut scheduler, mut event_rx) = scheduler();
        scheduler.schedule(TimerId::OverlapReveal, Duration::from_millis(20), 1);
        scheduler.cancel(TimerId::OverlapReveal);

        assert!(
            tokio::time::timeout(Duration::from_millis(60), event_rx.recv())
                .await
                .is_err()
        );
    }

    #[tokio::test]
    async fn shutdown_cancels_all_timers() {
        let (mut scheduler, mut event_rx) = scheduler();
        scheduler.schedule(TimerId::Alert, Duration::from_millis(20), 1);
        scheduler.schedule(TimerId::Welcome, Duration::from_millis(20), 1);

        scheduler.shutdown();

        assert!(
            tokio::time::timeout(Duration::from_millis(60), event_rx.recv())
                .await
                .is_err()
        );
    }
}
