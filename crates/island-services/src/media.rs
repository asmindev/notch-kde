use futures_lite::StreamExt;
use island_core::models::MediaTrack;
use std::collections::HashMap;
use tokio::sync::mpsc::UnboundedSender;
use tracing::{debug, error, info};
use zbus::zvariant::Value;
use zbus::Connection;
use zbus::MatchRule;
use zbus::MessageStream;

pub struct MediaService;

impl MediaService {
    pub async fn play_pause() {
        if let Ok(conn) = Connection::session().await {
            let _ = Self::call_player_method(&conn, "PlayPause").await;
        }
    }

    pub async fn next() {
        if let Ok(conn) = Connection::session().await {
            let _ = Self::call_player_method(&conn, "Next").await;
        }
    }

    pub async fn previous() {
        if let Ok(conn) = Connection::session().await {
            let _ = Self::call_player_method(&conn, "Previous").await;
        }
    }

    async fn call_player_method(conn: &Connection, method: &str) -> Result<(), zbus::Error> {
        let players = Self::find_players(conn).await?;
        if let Some(player) = players.first() {
            let proxy = zbus::Proxy::new(
                conn,
                player.as_str(),
                "/org/mpris/MediaPlayer2",
                "org.mpris.MediaPlayer2.Player",
            )
            .await?;
            let _: () = proxy.call(method, &()).await?;
        }
        Ok(())
    }

    async fn find_players(conn: &Connection) -> Result<Vec<String>, zbus::Error> {
        let dbus_proxy = zbus::fdo::DBusProxy::new(conn).await?;
        let names = dbus_proxy.list_names().await?;
        let mut players = Vec::new();
        for name in names {
            if name.starts_with("org.mpris.MediaPlayer2.") {
                players.push(name.to_string());
            }
        }
        Ok(players)
    }

    fn extract_str(val: &Value) -> Option<String> {
        match val {
            Value::Str(s) => Some(s.to_string()),
            Value::Value(v) => Self::extract_str(v),
            _ => None,
        }
    }

    fn extract_artist(val: &Value) -> Option<String> {
        match val {
            Value::Array(arr) => arr.first().and_then(Self::extract_str),
            Value::Str(s) => Some(s.to_string()),
            Value::Value(v) => Self::extract_artist(v),
            _ => None,
        }
    }

    fn url_decode(s: &str) -> String {
        let mut result = Vec::new();
        let bytes = s.as_bytes();
        let mut i = 0;
        while i < bytes.len() {
            if bytes[i] == b'%' && i + 2 < bytes.len() {
                if let Ok(hex_val) = u8::from_str_radix(&s[i + 1..i + 3], 16) {
                    result.push(hex_val);
                    i += 3;
                    continue;
                }
            }
            result.push(bytes[i]);
            i += 1;
        }
        String::from_utf8_lossy(&result).to_string()
    }

    async fn resolve_art_path(raw_url: &str) -> String {
        if raw_url.is_empty() {
            return String::new();
        }

        if let Some(stripped) = raw_url.strip_prefix("file://") {
            let decoded = Self::url_decode(stripped);
            if std::path::Path::new(&decoded).exists() {
                return decoded;
            }
        } else if raw_url.starts_with('/') {
            if std::path::Path::new(raw_url).exists() {
                return raw_url.to_string();
            }
        } else if raw_url.starts_with("http://") || raw_url.starts_with("https://") {
            let cache_dir = std::env::temp_dir().join("notchde-cache").join("albumart");
            let _ = std::fs::create_dir_all(&cache_dir);

            use std::hash::{DefaultHasher, Hash, Hasher};
            let mut hasher = DefaultHasher::new();
            raw_url.hash(&mut hasher);
            let hash = hasher.finish();
            let cache_file = cache_dir.join(format!("{:016x}.jpg", hash));

            if cache_file.exists() && std::fs::metadata(&cache_file).map(|m| m.len() > 0).unwrap_or(false) {
                return cache_file.to_string_lossy().to_string();
            }

            // Download using curl asynchronously with a quick timeout
            let res = tokio::process::Command::new("curl")
                .args(["-s", "-L", "--max-time", "4", "-o", cache_file.to_str().unwrap_or_default(), raw_url])
                .status()
                .await;

            if let Ok(st) = res {
                if st.success() && cache_file.exists() && std::fs::metadata(&cache_file).map(|m| m.len() > 0).unwrap_or(false) {
                    debug!("Downloaded album art to {:?}", cache_file);
                    return cache_file.to_string_lossy().to_string();
                }
            }
        }

        String::new()
    }

    async fn query_current_track(conn: &Connection) -> MediaTrack {
        if let Ok(players) = Self::find_players(conn).await {
            let mut best_candidate: Option<(String, HashMap<String, Value>, String)> = None;

            for player in &players {
                if let Ok(proxy) = zbus::Proxy::new(
                    conn,
                    player.as_str(),
                    "/org/mpris/MediaPlayer2",
                    "org.mpris.MediaPlayer2.Player",
                )
                .await
                {
                    let status: String = proxy
                        .get_property("PlaybackStatus")
                        .await
                        .unwrap_or_else(|_| "Stopped".to_string());

                    let metadata: HashMap<String, Value> = proxy
                        .get_property("Metadata")
                        .await
                        .unwrap_or_default();

                    let is_playing = status.eq_ignore_ascii_case("playing");

                    if is_playing {
                        best_candidate = Some((player.clone(), metadata, status));
                        break;
                    }

                    if best_candidate.is_none() && !metadata.is_empty() {
                        best_candidate = Some((player.clone(), metadata, status));
                    }
                }
            }

            if let Some((_player, metadata, status)) = best_candidate {
                let title = metadata
                    .get("xesam:title")
                    .and_then(Self::extract_str)
                    .unwrap_or_default();

                let artist = metadata
                    .get("xesam:artist")
                    .and_then(Self::extract_artist)
                    .unwrap_or_default();

                let album = metadata
                    .get("xesam:album")
                    .and_then(Self::extract_str)
                    .unwrap_or_default();

                let raw_art_url = metadata
                    .get("mpris:artUrl")
                    .or_else(|| metadata.get("xesam:artUrl"))
                    .and_then(Self::extract_str)
                    .unwrap_or_default();

                let art_url = Self::resolve_art_path(&raw_art_url).await;
                let is_playing = status.eq_ignore_ascii_case("playing");

                return MediaTrack {
                    title,
                    artist,
                    album,
                    art_url,
                    is_playing,
                    position_ms: 0,
                    duration_ms: 0,
                };
            }
        }
        MediaTrack::default()
    }

    pub fn start(sender: UnboundedSender<MediaTrack>) {
        tokio::spawn(async move {
            info!("MPRIS Media service starting with D-Bus event listening");
            let conn = match Connection::session().await {
                Ok(c) => c,
                Err(e) => {
                    error!("Failed to connect to D-Bus session bus: {}", e);
                    return;
                }
            };

            let mut last_track = MediaTrack::default();

            // Initial query
            let initial = Self::query_current_track(&conn).await;
            if initial != last_track {
                last_track = initial.clone();
                let _ = sender.send(initial);
            }

            // Match rules for D-Bus signals
            let prop_rule = MatchRule::builder()
                .msg_type(zbus::message::Type::Signal)
                .interface("org.freedesktop.DBus.Properties")
                .and_then(|b| b.member("PropertiesChanged"))
                .map(|b| b.build());

            let name_rule = MatchRule::builder()
                .msg_type(zbus::message::Type::Signal)
                .interface("org.freedesktop.DBus")
                .and_then(|b| b.member("NameOwnerChanged"))
                .map(|b| b.build());

            let mut prop_stream = match prop_rule {
                Ok(rule) => MessageStream::for_match_rule(rule, &conn, None).await.ok(),
                Err(e) => {
                    error!("Failed to create D-Bus PropertiesChanged match rule: {}", e);
                    None
                }
            };

            let mut name_stream = match name_rule {
                Ok(rule) => MessageStream::for_match_rule(rule, &conn, None).await.ok(),
                Err(e) => {
                    error!("Failed to create D-Bus NameOwnerChanged match rule: {}", e);
                    None
                }
            };

            info!("MPRIS Media service subscribed to D-Bus signals (zero polling)");

            loop {
                tokio::select! {
                    Some(_) = async {
                        match &mut prop_stream {
                            Some(s) => s.next().await,
                            None => std::future::pending().await,
                        }
                    } => {},
                    Some(_) = async {
                        match &mut name_stream {
                            Some(s) => s.next().await,
                            None => std::future::pending().await,
                        }
                    } => {},
                }

                let track = Self::query_current_track(&conn).await;
                if track != last_track {
                    debug!("Media track changed via D-Bus: {} - {}", track.artist, track.title);
                    last_track = track.clone();
                    if sender.send(track).is_err() {
                        break;
                    }
                }
            }
        });
    }
}
