use futures_lite::StreamExt;
use tokio::sync::mpsc::UnboundedSender;
use tracing::{info, warn};

pub struct NotificationService;

impl NotificationService {
    pub fn new() -> Self {
        Self
    }

    pub async fn listen(tx: UnboundedSender<()>) {
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
                &(&["type='method_call',interface='org.freedesktop.Notifications',member='Notify'"] as &[&str], 0u32),
            )
            .await
        {
            warn!("Could not start notification monitor: {err}");
            return;
        }

        info!("Notification monitor ready");
        let mut stream = zbus::MessageStream::from(connection);
        while let Some(message) = stream.next().await {
            let Ok(message) = message else {
                continue;
            };
            let header = message.header();
            if header.interface().is_some_and(|name| name == "org.freedesktop.Notifications")
                && header.member().is_some_and(|name| name == "Notify")
            {
                let _ = tx.send(());
            }
        }
    }
}
