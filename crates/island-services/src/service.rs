use island_core::models::{BatteryState, MediaUpdate, TimeInfo};
use island_core::prayer::PrayerAlert;
use std::collections::HashMap;
use std::future::Future;
use std::panic::{AssertUnwindSafe, catch_unwind};
use std::time::Duration;
use tokio::runtime::Handle;
use tokio::sync::{mpsc, watch};
use tokio::task::JoinHandle;
use tracing::{info, warn};

#[derive(Debug, Clone, PartialEq)]
pub enum DomainEvent {
    Tick(TimeInfo),
    Battery(BatteryState),
    Media(MediaUpdate),
    Dnd(bool),
    Muted(bool),
    Notification,
    Prayer(PrayerAlert),
    ScreenUnlocked,
    Spectrum([f32; 8]),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ServiceState {
    Starting,
    Running,
    Degraded,
    Stopped,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ServiceHealth {
    pub name: &'static str,
    pub state: ServiceState,
    pub detail: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ServiceError {
    pub service: &'static str,
    pub reason: String,
}

impl ServiceError {
    pub fn new(service: &'static str, reason: impl Into<String>) -> Self {
        Self {
            service,
            reason: reason.into(),
        }
    }
}

#[derive(Clone)]
struct ShutdownState {
    sender: watch::Sender<bool>,
}

#[derive(Clone)]
pub struct ShutdownSignal {
    state: std::sync::Arc<ShutdownState>,
}

impl ShutdownSignal {
    pub fn new() -> Self {
        let (sender, _) = watch::channel(false);
        Self {
            state: std::sync::Arc::new(ShutdownState { sender }),
        }
    }

    pub fn shutdown(&self) {
        self.state.sender.send_replace(true);
    }

    pub fn is_shutdown(&self) -> bool {
        *self.state.sender.borrow()
    }

    pub async fn cancelled(&self) {
        let mut receiver = self.state.sender.subscribe();
        if *receiver.borrow() {
            return;
        }
        while receiver.changed().await.is_ok() {
            if *receiver.borrow() {
                return;
            }
        }
    }
}

impl Default for ShutdownSignal {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Clone)]
pub struct ServiceContext {
    runtime: Handle,
    events: mpsc::UnboundedSender<DomainEvent>,
    shutdown: ShutdownSignal,
}

impl ServiceContext {
    pub fn new(
        runtime: Handle,
        events: mpsc::UnboundedSender<DomainEvent>,
        shutdown: ShutdownSignal,
    ) -> Self {
        Self {
            runtime,
            events,
            shutdown,
        }
    }

    pub fn emit(&self, event: DomainEvent) -> bool {
        self.events.send(event).is_ok()
    }

    pub fn event_sender(&self) -> mpsc::UnboundedSender<DomainEvent> {
        self.events.clone()
    }

    pub fn spawn(&self, future: impl Future<Output = ()> + Send + 'static) -> JoinHandle<()> {
        self.runtime.spawn(future)
    }

    pub fn spawn_blocking(&self, task: impl FnOnce() + Send + 'static) -> JoinHandle<()> {
        self.runtime.spawn_blocking(task)
    }

    pub fn shutdown_signal(&self) -> &ShutdownSignal {
        &self.shutdown
    }
}

pub struct ServiceHandle {
    pub name: &'static str,
    pub shutdown: ShutdownSignal,
    pub join: JoinHandle<()>,
}

impl ServiceHandle {
    pub fn new(name: &'static str, shutdown: ShutdownSignal, join: JoinHandle<()>) -> Self {
        Self {
            name,
            shutdown,
            join,
        }
    }
}

pub trait Service: Send + Sync + 'static {
    fn name(&self) -> &'static str;
    fn start(&self, ctx: ServiceContext) -> Result<ServiceHandle, ServiceError>;
}

pub struct ServiceSupervisor {
    services: Vec<Box<dyn Service>>,
    handles: Vec<ServiceHandle>,
    health: HashMap<&'static str, ServiceHealth>,
    shutdown: ShutdownSignal,
    shutdown_timeout: Duration,
    shutting_down: bool,
}

impl ServiceSupervisor {
    pub fn new(shutdown_timeout: Duration) -> Self {
        Self {
            services: Vec::new(),
            handles: Vec::new(),
            health: HashMap::new(),
            shutdown: ShutdownSignal::new(),
            shutdown_timeout,
            shutting_down: false,
        }
    }

    pub fn shutdown_signal(&self) -> ShutdownSignal {
        self.shutdown.clone()
    }

    pub fn register(&mut self, service: Box<dyn Service>) {
        let name = service.name();
        if self.health.contains_key(name) {
            warn!(service = name, "duplicate service name ignored");
            return;
        }
        self.set_health(name, ServiceState::Starting, None);
        self.services.push(service);
    }

    pub fn start_all(&mut self, ctx: &ServiceContext) {
        let services = std::mem::take(&mut self.services);
        for service in services {
            let name = service.name();
            match catch_unwind(AssertUnwindSafe(|| service.start(ctx.clone()))) {
                Ok(Ok(handle)) => {
                    self.handles.push(handle);
                    self.set_health(name, ServiceState::Running, None);
                }
                Ok(Err(err)) => {
                    warn!(
                        service = name,
                        reason = %err.reason,
                        "service failed to start"
                    );
                    self.set_health(name, ServiceState::Degraded, Some(err.reason));
                }
                Err(_) => {
                    warn!(service = name, "service panicked during start");
                    self.set_health(
                        name,
                        ServiceState::Degraded,
                        Some("panicked during start".to_string()),
                    );
                }
            }
        }
    }

    pub fn health(&self, name: &str) -> Option<&ServiceHealth> {
        self.health.get(name)
    }

    pub fn health_snapshot(&self) -> Vec<ServiceHealth> {
        let mut health: Vec<_> = self.health.values().cloned().collect();
        health.sort_by_key(|entry| entry.name);
        health
    }

    pub fn refresh_health(&mut self) {
        let finished: Vec<_> = self
            .handles
            .iter()
            .filter(|handle| handle.join.is_finished())
            .map(|handle| handle.name)
            .collect();

        for name in finished {
            if self.shutting_down {
                continue;
            }
            if self
                .health
                .get(name)
                .is_some_and(|health| health.state == ServiceState::Running)
            {
                warn!(service = name, "service stopped unexpectedly");
                self.set_health(
                    name,
                    ServiceState::Degraded,
                    Some("service task exited unexpectedly".to_string()),
                );
            }
        }
    }

    pub async fn shutdown(&mut self) {
        if self.shutting_down {
            return;
        }
        self.shutting_down = true;

        for handle in &self.handles {
            handle.shutdown.shutdown();
        }

        let mut handles = std::mem::take(&mut self.handles);
        for handle in &mut handles {
            match tokio::time::timeout(self.shutdown_timeout, &mut handle.join).await {
                Ok(Ok(())) => {
                    info!(service = handle.name, "service stopped");
                    self.set_health(handle.name, ServiceState::Stopped, None);
                }
                Ok(Err(err)) => {
                    warn!(service = handle.name, error = %err, "service task failed");
                    self.set_health(handle.name, ServiceState::Degraded, Some(err.to_string()));
                }
                Err(_) => {
                    warn!(
                        service = handle.name,
                        timeout_ms = self.shutdown_timeout.as_millis(),
                        "service shutdown timed out"
                    );
                    handle.join.abort();
                    self.set_health(
                        handle.name,
                        ServiceState::Degraded,
                        Some("shutdown timed out".to_string()),
                    );
                }
            }
        }
        self.handles = handles;
    }

    fn set_health(&mut self, name: &'static str, state: ServiceState, detail: Option<String>) {
        self.health.insert(
            name,
            ServiceHealth {
                name,
                state,
                detail,
            },
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct TestService {
        name: &'static str,
        fail_start: bool,
        exit_immediately: bool,
    }

    impl Service for TestService {
        fn name(&self) -> &'static str {
            self.name
        }

        fn start(&self, ctx: ServiceContext) -> Result<ServiceHandle, ServiceError> {
            if self.fail_start {
                return Err(ServiceError::new(self.name, "configured failure"));
            }

            let shutdown = ctx.shutdown_signal().clone();
            let task_shutdown = shutdown.clone();
            let exit_immediately = self.exit_immediately;
            let join = ctx.spawn(async move {
                if !exit_immediately {
                    task_shutdown.cancelled().await;
                }
            });
            Ok(ServiceHandle::new(self.name, shutdown, join))
        }
    }

    fn test_context() -> (
        ServiceContext,
        mpsc::UnboundedReceiver<DomainEvent>,
        ShutdownSignal,
    ) {
        let (event_tx, event_rx) = mpsc::unbounded_channel();
        let shutdown = ShutdownSignal::new();
        let ctx = ServiceContext::new(Handle::current(), event_tx, shutdown.clone());
        (ctx, event_rx, shutdown)
    }

    #[tokio::test]
    async fn starts_all_services() {
        let (ctx, _event_rx, _shutdown) = test_context();
        let mut supervisor = ServiceSupervisor::new(Duration::from_secs(1));
        supervisor.register(Box::new(TestService {
            name: "one",
            fail_start: false,
            exit_immediately: false,
        }));
        supervisor.register(Box::new(TestService {
            name: "two",
            fail_start: false,
            exit_immediately: false,
        }));

        supervisor.start_all(&ctx);

        assert_eq!(
            supervisor.health("one").unwrap().state,
            ServiceState::Running
        );
        assert_eq!(
            supervisor.health("two").unwrap().state,
            ServiceState::Running
        );
        supervisor.shutdown().await;
    }

    #[tokio::test]
    async fn start_failure_degrades_only_that_service() {
        let (ctx, _event_rx, _shutdown) = test_context();
        let mut supervisor = ServiceSupervisor::new(Duration::from_secs(1));
        supervisor.register(Box::new(TestService {
            name: "failing",
            fail_start: true,
            exit_immediately: false,
        }));
        supervisor.register(Box::new(TestService {
            name: "healthy",
            fail_start: false,
            exit_immediately: false,
        }));

        supervisor.start_all(&ctx);

        assert_eq!(
            supervisor.health("failing").unwrap().state,
            ServiceState::Degraded
        );
        assert_eq!(
            supervisor.health("healthy").unwrap().state,
            ServiceState::Running
        );
        supervisor.shutdown().await;
    }

    #[tokio::test]
    async fn runtime_failure_becomes_degraded() {
        let (ctx, _event_rx, _shutdown) = test_context();
        let mut supervisor = ServiceSupervisor::new(Duration::from_secs(1));
        supervisor.register(Box::new(TestService {
            name: "exits",
            fail_start: false,
            exit_immediately: true,
        }));

        supervisor.start_all(&ctx);
        tokio::task::yield_now().await;
        supervisor.refresh_health();

        assert_eq!(
            supervisor.health("exits").unwrap().state,
            ServiceState::Degraded
        );
        supervisor.shutdown().await;
    }

    #[tokio::test]
    async fn shutdown_stops_all_services() {
        let (ctx, _event_rx, shutdown) = test_context();
        let mut supervisor = ServiceSupervisor::new(Duration::from_secs(1));
        supervisor.register(Box::new(TestService {
            name: "one",
            fail_start: false,
            exit_immediately: false,
        }));
        supervisor.register(Box::new(TestService {
            name: "two",
            fail_start: false,
            exit_immediately: false,
        }));

        supervisor.start_all(&ctx);
        supervisor.shutdown().await;

        assert!(shutdown.is_shutdown());
        assert_eq!(
            supervisor.health("one").unwrap().state,
            ServiceState::Stopped
        );
        assert_eq!(
            supervisor.health("two").unwrap().state,
            ServiceState::Stopped
        );
    }
}
