use futures_lite::StreamExt;
use tokio::sync::mpsc::UnboundedSender;
use tracing::{debug, error, info};
use zbus::Connection;
use zbus::MatchRule;
use zbus::MessageStream;

pub struct ScreenSaverService;

impl ScreenSaverService {
    pub fn start(sender: UnboundedSender<()>) {
        tokio::spawn(async move {
            info!("ScreenSaver/Lockscreen listener starting");
            let conn = match Connection::session().await {
                Ok(c) => c,
                Err(e) => {
                    error!("Failed to connect to D-Bus session bus for screensaver: {}", e);
                    return;
                }
            };

            // Rule: org.freedesktop.ScreenSaver ActiveChanged(bool)
            let ss_rule = MatchRule::builder()
                .msg_type(zbus::message::Type::Signal)
                .interface("org.freedesktop.ScreenSaver")
                .and_then(|b| b.member("ActiveChanged"))
                .map(|b| b.build());

            let mut ss_stream = match ss_rule {
                Ok(rule) => MessageStream::for_match_rule(rule, &conn, None).await.ok(),
                Err(e) => {
                    error!("Failed to create ScreenSaver match rule: {}", e);
                    None
                }
            };

            info!("Subscribed to ScreenSaver ActiveChanged D-Bus signals");

            loop {
                tokio::select! {
                    Some(msg_res) = async {
                        match &mut ss_stream {
                            Some(s) => s.next().await,
                            None => std::future::pending().await,
                        }
                    } => {
                        if let Ok(msg) = msg_res {
                            if let Ok(body) = msg.body().deserialize::<bool>() {
                                let is_locked = body;
                                debug!("ScreenSaver ActiveChanged: is_locked = {}", is_locked);
                                if !is_locked {
                                    info!("Screen unlocked! Emitting Welcome event");
                                    let _ = sender.send(());
                                }
                            }
                        }
                    }
                }
            }
        });
    }
}
