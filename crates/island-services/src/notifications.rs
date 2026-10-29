use crate::service::{DomainEvent, Service, ServiceContext, ServiceError, ServiceHandle};
use futures_lite::StreamExt;
use tracing::{info, warn};

pub struct NotificationService;

impl NotificationService {
    pub fn new() -> Self {
        Self
    }
}

impl Default for NotificationService {
    fn default() -> Self {
        Self::new()
    }
}

impl Service for NotificationService {
    fn name(&self) -> &'static str {
        "notifications"
    }

    fn start(&self, ctx: ServiceContext) -> Result<ServiceHandle, ServiceError> {
        let shutdown = ctx.shutdown_signal().clone();
        let task_ctx = ctx.clone();
        let task_shutdown = shutdown.clone();
        let join = ctx.spawn(async move {
            let connection = match zbus::Connection::session().await {
                Ok(connection) => connection,
                Err(err) => {
                    warn!("Could not open session bus for notification monitor: {err}");
                    return;
                }
            };

            if let Err(err) = connection
                .call_method(
                    Some("org.freedesktop.DBus"),
                    "/org/freedesktop/DBus",
                    Some("org.freedesktop.DBus.Monitoring"),
                    "BecomeMonitor",
                    &(
                        &["type='method_call',interface='org.freedesktop.Notifications',member='Notify'"]
                            as &[&str],
                        0u32,
                    ),
                )
                .await
            {
                warn!("Could not start notification monitor: {err}");
                return;
            }

            info!("Notification monitor ready");
            let mut stream = zbus::MessageStream::from(connection);
            loop {
                tokio::select! {
                    _ = task_shutdown.cancelled() => break,
                    message = stream.next() => {
                        let Some(Ok(message)) = message else { break };
                        let header = message.header();
                        if header.interface().is_some_and(|name| name == "org.freedesktop.Notifications")
                            && header.member().is_some_and(|name| name == "Notify")
                            && !task_ctx.emit(DomainEvent::Notification)
                        {
                            break;
                        }
                    }
                }
            }
        });
        Ok(ServiceHandle::new(self.name(), shutdown, join))
    }
}
